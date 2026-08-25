pub mod kernel;
pub mod profiles;
pub mod score;
pub mod settings;
pub mod templates;

use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    ok: bool,
    message: String,
    data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(message: String, data: Option<T>) -> Self {
        Self {
            ok: true,
            message,
            data,
        }
    }

    pub fn failure(message: String, data: Option<T>) -> Self {
        Self {
            ok: false,
            message,
            data,
        }
    }
}

pub fn simple_response<T>(result: Result<T, String>) -> Json<ApiResponse<T>> {
    match result {
        Ok(data) => Json(ApiResponse::success(String::new(), Some(data))),
        Err(err) => Json(ApiResponse::failure(err, None)),
    }
}
