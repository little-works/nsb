use axum::Json;
use axum::extract::{Multipart, State};
use serde::{Deserialize, Serialize};

use crate::app::data_transfer::{
    DataImportReport, PortableDataArchive, export_archive, plan_import,
};
use crate::config::{AppConfig, AppLanguage};
use crate::hosts::AutoLaunchHost;
use crate::routes::RouteState;

use super::{ApiResponse, simple_response};

#[derive(Debug, Deserialize)]
pub struct SaveSettingsRequest {
    app_port: u16,
    #[serde(default)]
    system_proxy_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAutoLaunchRequest {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SaveAppLanguageRequest {
    app_language: AppLanguage,
}

#[derive(Clone, Serialize)]
pub struct SettingsResponse {
    pub app_port: u16,
    pub system_proxy_enabled: bool,
    pub app_language: AppLanguage,
}

fn settings_response(config: &AppConfig) -> SettingsResponse {
    SettingsResponse {
        app_port: config.app_port,
        system_proxy_enabled: config.system_proxy_enabled,
        app_language: config.app_language,
    }
}

pub async fn get_settings(State(ctx): State<RouteState>) -> Json<ApiResponse<SettingsResponse>> {
    let guard = ctx.runtime.lock().await;
    Json(ApiResponse::success(
        String::new(),
        Some(settings_response(&guard.controller.state.gui_config)),
    ))
}

pub async fn get_auto_launch() -> Json<ApiResponse<bool>> {
    simple_response(run_auto_launch(AutoLaunchHost::is_enabled).await)
}

pub async fn update_auto_launch(
    Json(request): Json<UpdateAutoLaunchRequest>,
) -> Json<ApiResponse<bool>> {
    simple_response(run_auto_launch(move || AutoLaunchHost::set_enabled(request.enabled)).await)
}

pub async fn save_settings(
    State(ctx): State<RouteState>,
    Json(request): Json<SaveSettingsRequest>,
) -> Json<ApiResponse<SettingsResponse>> {
    let mut guard = ctx.runtime.lock().await;
    match guard
        .save_runtime_settings(request.app_port, request.system_proxy_enabled)
        .await
    {
        Ok(()) => {
            let snapshot = settings_response(&guard.controller.state.gui_config);
            Json(ApiResponse::success(
                String::from("Runtime settings saved."),
                Some(snapshot),
            ))
        }
        Err(err) => {
            let snapshot = settings_response(&guard.controller.state.gui_config);
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn save_app_language(
    State(ctx): State<RouteState>,
    Json(request): Json<SaveAppLanguageRequest>,
) -> Json<ApiResponse<SettingsResponse>> {
    let mut guard = ctx.runtime.lock().await;
    match guard.save_app_language(request.app_language).await {
        Ok(()) => {
            let system_proxy_enabled = guard.controller.state.gui_config.system_proxy_enabled;
            let snapshot = settings_response(&guard.controller.state.gui_config);
            drop(guard);
            if ctx
                .app_action_proxy
                .send_event(crate::app::AppAction::RefreshTrayLabels {
                    app_language: request.app_language,
                    system_proxy_enabled,
                })
                .is_err()
            {
                crate::route_log!(
                    ctx,
                    "warn",
                    "failed to refresh tray labels after saving app language"
                );
            }
            Json(ApiResponse::success(
                String::from("Application language saved."),
                Some(snapshot),
            ))
        }
        Err(err) => {
            crate::route_log!(ctx, "warn", "failed to save application language: {err}");
            let snapshot = settings_response(&guard.controller.state.gui_config);
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn export_portable_data(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<PortableDataArchive>> {
    let guard = ctx.runtime.lock().await;
    Json(ApiResponse::success(
        String::new(),
        Some(export_archive(&guard.controller.state.gui_config)),
    ))
}

pub async fn preview_portable_data(
    State(ctx): State<RouteState>,
    multipart: Multipart,
) -> Json<ApiResponse<DataImportReport>> {
    let content = match read_import_file(multipart).await {
        Ok(content) => content,
        Err(error) => return Json(ApiResponse::failure(error, None)),
    };
    let guard = ctx.runtime.lock().await;
    match plan_import(&content, &guard.controller.state.gui_config) {
        Ok(plan) => Json(ApiResponse::success(String::new(), Some(plan.report))),
        Err(error) => Json(ApiResponse::failure(error, None)),
    }
}

pub async fn import_portable_data(
    State(ctx): State<RouteState>,
    multipart: Multipart,
) -> Json<ApiResponse<DataImportReport>> {
    let content = match read_import_file(multipart).await {
        Ok(content) => content,
        Err(error) => return Json(ApiResponse::failure(error, None)),
    };
    let mut guard = ctx.runtime.lock().await;
    let plan = match plan_import(&content, &guard.controller.state.gui_config) {
        Ok(plan) => plan,
        Err(error) => return Json(ApiResponse::failure(error, None)),
    };
    if plan.report.actionable_count() == 0 {
        return Json(ApiResponse::success(String::new(), Some(plan.report)));
    }
    if let Err(error) = guard.app_config_store.save_all(&plan.config).await {
        return Json(ApiResponse::failure(error, None));
    }

    guard.controller.state.gui_config = plan.config;
    let profile_host = guard.profile_host.clone();
    for id in &plan.imported_profile_ids {
        let _ = profile_host.delete_runtime(id).await;
    }
    Json(ApiResponse::success(
        String::from("Templates and Profiles imported."),
        Some(plan.report),
    ))
}

async fn read_import_file(mut multipart: Multipart) -> Result<String, String> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| format!("Failed to parse uploaded data file: {error}"))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|error| format!("Failed to read imported data file: {error}"))?;
        if bytes.is_empty() {
            return Err(String::from("Imported data file is empty."));
        }
        return String::from_utf8(bytes.to_vec())
            .map_err(|_| String::from("Imported data file must be UTF-8 JSON."));
    }
    Err(String::from("Select a JSON data backup file."))
}

async fn run_auto_launch(
    operation: impl FnOnce() -> Result<bool, String> + Send + 'static,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|err| format!("Auto-launch operation failed: {err}"))?
}
