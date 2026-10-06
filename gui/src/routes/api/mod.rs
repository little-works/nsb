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
    #[serde(skip_serializing_if = "Option::is_none")]
    message_level: Option<ApiMessageLevel>,
    data: Option<T>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiMessageLevel {
    Info,
    Warn,
    Error,
}

impl<T> ApiResponse<T> {
    pub fn success(message: String, data: Option<T>) -> Self {
        Self {
            ok: true,
            message,
            message_level: None,
            data,
        }
    }

    pub fn success_with_level(
        message: String,
        data: Option<T>,
        message_level: ApiMessageLevel,
    ) -> Self {
        Self {
            ok: true,
            message,
            message_level: Some(message_level),
            data,
        }
    }

    pub fn failure(message: String, data: Option<T>) -> Self {
        Self {
            ok: false,
            message,
            message_level: None,
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
