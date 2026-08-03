use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use crate::app::AppSnapshot;
use crate::config::AppLanguage;
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
) -> Json<ApiResponse<AppSnapshot>> {
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
            let snapshot = guard.snapshot().await;
            Json(ApiResponse::success(
                String::from("Runtime settings saved."),
                Some(snapshot),
            ))
        }
        Err(err) => {
            let snapshot = guard.snapshot().await;
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn save_app_language(
    State(ctx): State<RouteState>,
    Json(request): Json<SaveAppLanguageRequest>,
) -> Json<ApiResponse<AppSnapshot>> {
    let mut guard = ctx.runtime.lock().await;
    match guard.save_app_language(request.app_language).await {
        Ok(()) => {
            let system_proxy_enabled = guard.controller.state.gui_config.system_proxy_enabled;
            let snapshot = guard.snapshot().await;
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
            let snapshot = guard.snapshot().await;
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
