use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::app::update_profile_runtime;
use crate::routes::RouteState;
use crate::state::{ProfileHeader, ProfileItem, ProfileKind, ProfileRemote};

use super::ApiResponse;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProfileListResponse {
    pub profiles: Vec<ProfileSummary>,
    pub current_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub kind: ProfileKind,
    pub updated_at: u64,
    pub last_attempt_at: u64,
    pub last_update_failed: bool,
}

impl From<&ProfileItem> for ProfileSummary {
    fn from(profile: &ProfileItem) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            kind: profile.kind.clone(),
            updated_at: profile.updated_at,
            last_attempt_at: profile.last_attempt_at,
            last_update_failed: profile.last_update_error.is_some(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    name: String,
    source: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    headers: Vec<ProfileHeader>,
    #[serde(default)]
    update_interval_hours: Option<u32>,
    #[serde(default)]
    update_cron: Option<String>,
    #[serde(default)]
    remotes: Vec<ProfileRemote>,
    #[serde(default)]
    hook: Option<String>,
    #[serde(default)]
    keep_subscription_groups_and_rules: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    name: String,
    source: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    headers: Vec<ProfileHeader>,
    #[serde(default)]
    update_interval_hours: Option<u32>,
    #[serde(default)]
    update_cron: Option<String>,
    #[serde(default)]
    remotes: Vec<ProfileRemote>,
    #[serde(default)]
    hook: Option<String>,
    #[serde(default)]
    keep_subscription_groups_and_rules: bool,
}

#[derive(Debug, Deserialize)]
pub struct ImportProfileRequest {
    file_name: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveProfileContentRequest {
    content: String,
}

pub async fn list_profiles(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let guard = ctx.runtime.lock().await;
    Json(ApiResponse::success(
        String::new(),
        Some(profile_list_response(&guard)),
    ))
}

pub async fn get_profile(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileItem>> {
    let guard = ctx.runtime.lock().await;
    match guard
        .controller
        .state
        .gui_config
        .profiles
        .iter()
        .find(|profile| profile.id == id)
    {
        Some(profile) => Json(ApiResponse::success(String::new(), Some(profile.clone()))),
        None => Json(ApiResponse::failure(
            String::from("Specified Profile was not found."),
            None,
        )),
    }
}

pub async fn refresh_profile(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let current_profile_id = {
        let guard = ctx.runtime.lock().await;
        guard.controller.state.gui_config.current_profile_id.clone()
    };
    let Some(profile_id) = current_profile_id else {
        return Json(ApiResponse::failure(
            String::from("No Profile is active. Add and select a Profile first."),
            None,
        ));
    };

    // The refreshed runtime is the source for the active kernel. Apply it so the
    // workspace config is regenerated and the core is running after refresh.
    match update_profile_runtime(ctx.runtime.clone(), profile_id, true).await {
        Ok(()) => {
            let guard = ctx.runtime.lock().await;
            let snapshot = profile_list_response(&guard);
            Json(ApiResponse::success(
                String::from("Current Profile refreshed."),
                Some(snapshot),
            ))
        }
        Err(err) => {
            let guard = ctx.runtime.lock().await;
            let snapshot = profile_list_response(&guard);
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn refresh_profile_by_id(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileListResponse>> {
    // `update_profile_runtime` applies the refreshed runtime only when this Profile
    // is active, so refreshing an inactive Profile remains side-effect free for the
    // kernel.
    match update_profile_runtime(ctx.runtime.clone(), id, true).await {
        Ok(()) => {
            let guard = ctx.runtime.lock().await;
            let snapshot = profile_list_response(&guard);
            Json(ApiResponse::success(
                String::from("Profile refreshed."),
                Some(snapshot),
            ))
        }
        Err(err) => {
            let guard = ctx.runtime.lock().await;
            let snapshot = profile_list_response(&guard);
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn import_profile(
    State(ctx): State<RouteState>,
    Json(request): Json<ImportProfileRequest>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let mut guard = ctx.runtime.lock().await;
    let file_name = request.file_name.trim();
    let base_name = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Profile");
    let name = unique_profile_name(base_name, &guard.controller.state.gui_config.profiles);
    let should_select = guard
        .controller
        .state
        .gui_config
        .current_profile_id
        .is_none();

    match guard
        .create_profile(
            name.clone(),
            String::new(),
            Some(request.content),
            Vec::new(),
            None,
            None,
        )
        .await
    {
        Ok(()) => {
            if should_select {
                let created_id = guard
                    .controller
                    .state
                    .gui_config
                    .profiles
                    .last()
                    .map(|profile| profile.id.clone());
                if let Some(created_id) = created_id {
                    if let Err(err) = guard.activate_profile(created_id).await {
                        return Json(ApiResponse::failure(
                            err,
                            Some(profile_list_response(&guard)),
                        ));
                    }
                }
            }
            Json(ApiResponse::success(
                format!("Profile imported: {name}."),
                Some(profile_list_response(&guard)),
            ))
        }
        Err(err) => Json(ApiResponse::failure(
            err,
            Some(profile_list_response(&guard)),
        )),
    }
}

pub async fn create_profile(
    State(ctx): State<RouteState>,
    Json(request): Json<CreateProfileRequest>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let remotes = request.remotes;
    let hook = request.hook;
    let source = remotes
        .first()
        .map(|remote| remote.url.clone())
        .unwrap_or(request.source);
    let headers = remotes
        .first()
        .map(|remote| remote.headers.clone())
        .unwrap_or(request.headers);
    let (created, should_select_after_download) = {
        let mut guard = ctx.runtime.lock().await;
        let had_current_profile = guard
            .controller
            .state
            .gui_config
            .current_profile_id
            .is_some();
        let result = guard
            .create_profile(
                request.name,
                source,
                request.content,
                headers,
                request.update_interval_hours,
                request.update_cron,
            )
            .await;
        match result {
            Ok(()) => {
                let created = guard.controller.state.gui_config.profiles.last().cloned();
                if let Some(created) = &created {
                    let crate::app::GuiRuntime {
                        controller,
                        app_config_store,
                        ..
                    } = &mut *guard;
                    if let Err(err) = controller
                        .configure_profile_sources(
                            &created.id,
                            remotes,
                            hook,
                            request.keep_subscription_groups_and_rules,
                            app_config_store,
                        )
                        .await
                    {
                        return Json(ApiResponse::failure(
                            err,
                            Some(profile_list_response(&guard)),
                        ));
                    }
                }
                (created, !had_current_profile)
            }
            Err(err) => {
                return Json(ApiResponse::failure(
                    err,
                    Some(profile_list_response(&guard)),
                ));
            }
        }
    };

    let Some(created) = created else {
        return Json(ApiResponse::failure(
            String::from("Failed to create Profile."),
            None,
        ));
    };

    let mut message = String::from("Profile added.");
    if matches!(created.kind, ProfileKind::File) && should_select_after_download {
        let mut guard = ctx.runtime.lock().await;
        if let Err(err) = guard.activate_profile(created.id.clone()).await {
            return Json(ApiResponse::failure(
                err,
                Some(profile_list_response(&guard)),
            ));
        }
    } else if matches!(created.kind, ProfileKind::Url) {
        if should_select_after_download {
            match update_profile_runtime(ctx.runtime.clone(), created.id.clone(), false).await {
                Ok(()) => {
                    let mut guard = ctx.runtime.lock().await;
                    if let Err(err) = guard.activate_profile(created.id.clone()).await {
                        return Json(ApiResponse::failure(
                            err,
                            Some(profile_list_response(&guard)),
                        ));
                    }
                }
                Err(err) => message = format!("Profile added, but initial download failed: {err}"),
            }
        } else {
            let runtime = ctx.runtime.clone();
            let profile_id = created.id.clone();
            tokio::spawn(async move {
                let _ = update_profile_runtime(runtime, profile_id, false).await;
            });
        }
    }

    let guard = ctx.runtime.lock().await;
    Json(ApiResponse::success(
        message,
        Some(profile_list_response(&guard)),
    ))
}

pub async fn update_profile(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
    Json(request): Json<UpdateProfileRequest>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let remotes = request.remotes;
    let hook = request.hook;
    let source = remotes
        .first()
        .map(|remote| remote.url.clone())
        .unwrap_or(request.source);
    let headers = remotes
        .first()
        .map(|remote| remote.headers.clone())
        .unwrap_or(request.headers);
    let mut guard = ctx.runtime.lock().await;
    match guard
        .update_profile(
            id.clone(),
            request.name,
            source,
            request.content,
            headers,
            request.update_interval_hours,
            request.update_cron,
        )
        .await
    {
        Ok(()) => {
            let crate::app::GuiRuntime {
                controller,
                app_config_store,
                ..
            } = &mut *guard;
            if let Err(err) = controller
                .configure_profile_sources(
                    &id,
                    remotes,
                    hook,
                    request.keep_subscription_groups_and_rules,
                    app_config_store,
                )
                .await
            {
                return Json(ApiResponse::failure(
                    err,
                    Some(profile_list_response(&guard)),
                ));
            }
            Json(ApiResponse::success(
                String::from("Profile updated."),
                Some(profile_list_response(&guard)),
            ))
        }
        Err(err) => Json(ApiResponse::failure(
            err,
            Some(profile_list_response(&guard)),
        )),
    }
}

pub async fn delete_profile(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let mut guard = ctx.runtime.lock().await;
    match guard.delete_profile(id).await {
        Ok(()) => Json(ApiResponse::success(
            String::from("Profile deleted."),
            Some(profile_list_response(&guard)),
        )),
        Err(err) => Json(ApiResponse::failure(
            err,
            Some(profile_list_response(&guard)),
        )),
    }
}

pub async fn set_current_profile(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<ProfileListResponse>> {
    let mut guard = ctx.runtime.lock().await;
    match guard.activate_profile(id).await {
        Ok(()) => {
            let snapshot = profile_list_response(&guard);
            Json(ApiResponse::success(String::new(), Some(snapshot)))
        }
        Err(err) => {
            let snapshot = profile_list_response(&guard);
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn get_profile_content(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<String>> {
    let guard = ctx.runtime.lock().await;
    match guard.read_profile_runtime(&id).await {
        Ok(content) => Json(ApiResponse::success(String::new(), Some(content))),
        Err(err) => Json(ApiResponse::failure(err, None)),
    }
}

pub async fn save_profile_content(
    Path(id): Path<String>,
    State(ctx): State<RouteState>,
    Json(request): Json<SaveProfileContentRequest>,
) -> Json<ApiResponse<String>> {
    let mut guard = ctx.runtime.lock().await;
    match guard.save_profile_runtime(&id, request.content).await {
        Ok(()) => Json(ApiResponse::success(
            String::from("Profile node configuration saved."),
            Some(String::new()),
        )),
        Err(err) => Json(ApiResponse::failure(err, None)),
    }
}

fn profile_list_response(runtime: &crate::app::GuiRuntime) -> ProfileListResponse {
    ProfileListResponse {
        profiles: runtime
            .controller
            .state
            .gui_config
            .profiles
            .iter()
            .map(ProfileSummary::from)
            .collect(),
        current_profile_id: runtime
            .controller
            .state
            .gui_config
            .current_profile_id
            .clone(),
    }
}

fn unique_profile_name(base_name: &str, profiles: &[ProfileItem]) -> String {
    if !profiles.iter().any(|profile| profile.name == base_name) {
        return base_name.to_string();
    }

    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{base_name} {suffix}");
        if !profiles.iter().any(|profile| profile.name == candidate) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_summary_excludes_subscription_secrets() {
        let profile = ProfileItem {
            id: String::from("profile-1"),
            name: String::from("Private subscription"),
            kind: ProfileKind::Url,
            url: String::from("https://example.test/subscribe?token=secret"),
            updated_at: 10,
            headers: vec![ProfileHeader {
                key: String::from("Authorization"),
                value: String::from("Bearer secret"),
            }],
            remotes: vec![ProfileRemote {
                name: String::from("Remote"),
                url: String::from("https://example.test/remote?token=secret"),
                headers: Vec::new(),
            }],
            hook: Some(String::from("secret hook")),
            keep_subscription_groups_and_rules: true,
            update_interval_hours: Some(24),
            update_cron: None,
            next_update_at: 20,
            last_attempt_at: 15,
            last_update_error: Some(String::from("request failed: token=secret")),
            revision: 1,
        };

        let summary = ProfileSummary::from(&profile);
        let value = serde_json::to_value(summary).expect("summary serializes");

        assert_eq!(value["last_update_failed"], true);
        for secret in ["token", "Authorization", "hook", "last_update_error"] {
            assert!(value.get(secret).is_none(), "summary exposed {secret}");
        }
        assert!(!value.to_string().contains("secret"));
    }
}
