use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;

use crate::routes::RouteState;
use crate::state::{ProfileTemplate, current_timestamp, generate_profile_id};

use super::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct SaveTemplateRequest {
    pub name: String,
    pub content: String,
}

fn validate(request: SaveTemplateRequest) -> Result<(String, String), String> {
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return Err(String::from("Template name cannot be empty."));
    }
    let value: serde_json::Value = serde_json::from_str(&request.content)
        .map_err(|error| format!("Template must be valid sing-box JSON: {error}"))?;
    if !value.is_object() {
        return Err(String::from(
            "Template sing-box configuration must be a JSON object.",
        ));
    }
    serde_json::from_value::<nsb_core::SingBoxConfig>(value.clone())
        .map_err(|error| format!("Template must be structurally valid sing-box JSON: {error}"))?;
    Ok((
        name,
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?,
    ))
}

pub async fn list_templates(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<Vec<ProfileTemplate>>> {
    let guard = ctx.runtime.lock().await;
    let templates = guard
        .controller
        .state
        .gui_config
        .templates
        .iter()
        .cloned()
        .map(|mut template| {
            template.reference_count = guard
                .controller
                .state
                .gui_config
                .profiles
                .iter()
                .filter(|profile| profile.template_id == template.id)
                .count();
            template
        })
        .collect();
    Json(ApiResponse::success(String::new(), Some(templates)))
}

pub async fn default_template() -> Json<ApiResponse<String>> {
    match serde_json::to_string_pretty(&nsb_core::default_template()) {
        Ok(content) => Json(ApiResponse::success(String::new(), Some(content))),
        Err(error) => Json(ApiResponse::failure(
            format!("Failed to serialize the default Template: {error}"),
            None,
        )),
    }
}

pub async fn get_template(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileTemplate>> {
    let guard = ctx.runtime.lock().await;
    match guard
        .controller
        .state
        .gui_config
        .templates
        .iter()
        .find(|item| item.id == id)
        .cloned()
    {
        Some(mut item) => {
            item.reference_count = guard
                .controller
                .state
                .gui_config
                .profiles
                .iter()
                .filter(|profile| profile.template_id == item.id)
                .count();
            Json(ApiResponse::success(String::new(), Some(item)))
        }
        None => Json(ApiResponse::failure(
            String::from("Specified Template was not found."),
            None,
        )),
    }
}
pub async fn create_template(
    State(ctx): State<RouteState>,
    Json(request): Json<SaveTemplateRequest>,
) -> Json<ApiResponse<ProfileTemplate>> {
    let (name, content) = match validate(request) {
        Ok(value) => value,
        Err(error) => return Json(ApiResponse::failure(error, None)),
    };
    let mut guard = ctx.runtime.lock().await;
    let item = ProfileTemplate {
        id: generate_profile_id(),
        name,
        content,
        updated_at: current_timestamp(),
        reference_count: 0,
    };
    guard
        .controller
        .state
        .gui_config
        .templates
        .push(item.clone());
    match guard
        .app_config_store
        .save(&guard.controller.state.gui_config)
        .await
    {
        Ok(()) => Json(ApiResponse::success(
            String::from("Template created."),
            Some(item),
        )),
        Err(error) => Json(ApiResponse::failure(error, None)),
    }
}
pub async fn update_template(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
    Json(request): Json<SaveTemplateRequest>,
) -> Json<ApiResponse<ProfileTemplate>> {
    let (name, content) = match validate(request) {
        Ok(value) => value,
        Err(error) => return Json(ApiResponse::failure(error, None)),
    };
    let mut guard = ctx.runtime.lock().await;
    let Some(index) = guard
        .controller
        .state
        .gui_config
        .templates
        .iter()
        .position(|item| item.id == id)
    else {
        return Json(ApiResponse::failure(
            String::from("Specified Template was not found."),
            None,
        ));
    };
    let item = &mut guard.controller.state.gui_config.templates[index];
    item.name = name;
    item.content = content;
    item.updated_at = current_timestamp();
    let response = item.clone();
    match guard
        .app_config_store
        .save(&guard.controller.state.gui_config)
        .await
    {
        Ok(()) => Json(ApiResponse::success(
            String::from("Template saved."),
            Some(response),
        )),
        Err(error) => Json(ApiResponse::failure(error, None)),
    }
}
pub async fn delete_template(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<()>> {
    let mut guard = ctx.runtime.lock().await;
    let references = guard
        .controller
        .state
        .gui_config
        .profiles
        .iter()
        .filter(|profile| profile.template_id == id)
        .count();
    if references > 0 {
        return Json(ApiResponse::failure(
            format!("Template is used by {references} Profile(s) and cannot be deleted."),
            None,
        ));
    }
    let Some(index) = guard
        .controller
        .state
        .gui_config
        .templates
        .iter()
        .position(|item| item.id == id)
    else {
        return Json(ApiResponse::failure(
            String::from("Specified Template was not found."),
            None,
        ));
    };
    guard.controller.state.gui_config.templates.remove(index);
    match guard
        .app_config_store
        .save(&guard.controller.state.gui_config)
        .await
    {
        Ok(()) => Json(ApiResponse::success(
            String::from("Template deleted."),
            Some(()),
        )),
        Err(error) => Json(ApiResponse::failure(error, None)),
    }
}
