use axum::Json;
use axum::extract::State;

use crate::app::AppSnapshot;
use crate::routes::RouteState;

use super::ApiResponse;

pub async fn get_state(State(ctx): State<RouteState>) -> Json<ApiResponse<AppSnapshot>> {
    let mut guard = ctx.runtime.lock().await;
    Json(ApiResponse::success(
        String::new(),
        Some(guard.snapshot().await),
    ))
}
