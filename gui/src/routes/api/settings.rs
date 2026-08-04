use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, AppLanguage};
use crate::hosts::AutoLaunchHost;
use crate::routes::RouteState;

use super::{ApiResponse, simple_response};

#[derive(Debug, Deserialize)]
pub struct SaveSettingsRequest {
    mixed_port: u16,
    app_port: u16,
    allow_lan: bool,
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
    pub mixed_port: u16,
    pub app_port: u16,
    pub allow_lan: bool,
    pub system_proxy_enabled: bool,
    pub app_language: AppLanguage,
}

fn settings_response(config: &AppConfig) -> SettingsResponse {
    SettingsResponse {
        mixed_port: config.mixed_port,
        app_port: config.app_port,
        allow_lan: config.allow_lan,
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
        .save_runtime_settings(
            request.mixed_port,
            request.app_port,
            request.allow_lan,
            request.system_proxy_enabled,
        )
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

async fn run_auto_launch(
    operation: impl FnOnce() -> Result<bool, String> + Send + 'static,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|err| format!("Auto-launch operation failed: {err}"))?
}
