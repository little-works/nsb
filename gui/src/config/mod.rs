use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;
use ts_rs::TS;

use crate::state::{ProfileItem, ProfileTemplate};
use crate::utils::path::{ensure_config_dir, ensure_data_dir};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AppLanguage {
    #[serde(rename = "en-US")]
    #[ts(rename = "en-US")]
    EnUs,
    #[serde(rename = "zh-CN")]
    #[ts(rename = "zh-CN")]
    ZhCn,
}

impl Default for AppLanguage {
    fn default() -> Self {
        default_app_language()
    }
}

fn default_app_language() -> AppLanguage {
    #[cfg(windows)]
    {
        const LOCALE_NAME_MAX_LENGTH: usize = 85;
        let mut locale = [0_u16; LOCALE_NAME_MAX_LENGTH];
        let length = unsafe {
            windows_sys::Win32::Globalization::GetUserDefaultLocaleName(
                locale.as_mut_ptr(),
                locale.len() as i32,
            )
        };
        if length > 1 {
            let locale = String::from_utf16_lossy(&locale[..length as usize - 1]);
            if locale.starts_with("zh") {
                return AppLanguage::ZhCn;
            }
        }
    }

    #[cfg(not(windows))]
    if std::env::var("LANG")
        .ok()
        .is_some_and(|locale| locale.starts_with("zh"))
    {
        return AppLanguage::ZhCn;
    }

    AppLanguage::EnUs
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AppConfig {
    #[serde(default = "default_app_port")]
    pub app_port: u16,
    #[serde(default)]
    pub system_proxy_enabled: bool,
    #[serde(default)]
    pub app_language: AppLanguage,
    #[serde(default)]
    pub profiles: Vec<ProfileItem>,
    #[serde(default)]
    pub templates: Vec<ProfileTemplate>,
    #[serde(default)]
    pub current_profile_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_port: default_app_port(),
            system_proxy_enabled: false,
            app_language: AppLanguage::default(),
            profiles: Vec::new(),
            templates: Vec::new(),
            current_profile_id: None,
        }
    }
}

fn default_app_port() -> u16 {
    if cfg!(debug_assertions) { 18787 } else { 8787 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableConfig {
    #[serde(default = "default_app_port")]
    app_port: u16,
    #[serde(default)]
    system_proxy_enabled: bool,
    #[serde(default)]
    app_language: AppLanguage,
    #[serde(default)]
    profiles: Vec<PortableProfile>,
    #[serde(default)]
    templates: Vec<PortableTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableProfile {
    id: String,
    name: String,
    #[serde(default)]
    template_id: String,
    #[serde(default)]
    inline_template: Option<String>,
    #[serde(default)]
    remotes: Vec<crate::state::ProfileRemote>,
    #[serde(default)]
    hook: Option<String>,
    #[serde(default)]
    update_interval_hours: Option<u32>,
    #[serde(default)]
    update_cron: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortableTemplate {
    id: String,
    name: String,
    content: String,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RuntimeState {
    #[serde(default)]
    current_profile_id: Option<String>,
    #[serde(default)]
    profiles: BTreeMap<String, ProfileRuntimeState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileRuntimeState {
    #[serde(default)]
    updated_at: u64,
    #[serde(default)]
    next_update_at: u64,
    #[serde(default)]
    last_attempt_at: u64,
    #[serde(default)]
    last_update_error: Option<String>,
    #[serde(default)]
    revision: u64,
}

impl From<&AppConfig> for PortableConfig {
    fn from(config: &AppConfig) -> Self {
        Self {
            app_port: config.app_port,
            system_proxy_enabled: config.system_proxy_enabled,
            app_language: config.app_language,
            profiles: config.profiles.iter().map(PortableProfile::from).collect(),
            templates: config
                .templates
                .iter()
                .map(PortableTemplate::from)
                .collect(),
        }
    }
}

impl From<&ProfileItem> for PortableProfile {
    fn from(profile: &ProfileItem) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            template_id: profile.template_id.clone(),
            inline_template: profile.inline_template.clone(),
            remotes: profile.remotes.clone(),
            hook: profile.hook.clone(),
            update_interval_hours: profile.update_interval_hours,
            update_cron: profile.update_cron.clone(),
        }
    }
}

impl From<&ProfileTemplate> for PortableTemplate {
    fn from(template: &ProfileTemplate) -> Self {
        Self {
            id: template.id.clone(),
            name: template.name.clone(),
            content: template.content.clone(),
            updated_at: template.updated_at,
        }
    }
}

impl From<PortableProfile> for ProfileItem {
    fn from(profile: PortableProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            template_id: profile.template_id,
            inline_template: profile.inline_template,
            updated_at: 0,
            remotes: profile.remotes,
            hook: profile.hook,
            update_interval_hours: profile.update_interval_hours,
            update_cron: profile.update_cron,
            next_update_at: 0,
            last_attempt_at: 0,
            last_update_error: None,
            revision: 0,
        }
    }
}

impl From<PortableTemplate> for ProfileTemplate {
    fn from(template: PortableTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            content: template.content,
            updated_at: template.updated_at,
            reference_count: 0,
        }
    }
}

impl From<&AppConfig> for RuntimeState {
    fn from(config: &AppConfig) -> Self {
        Self {
            current_profile_id: config
                .current_profile_id
                .as_ref()
                .filter(|id| config.profiles.iter().any(|profile| profile.id == **id))
                .cloned(),
            profiles: config
                .profiles
                .iter()
                .map(|profile| {
                    (
                        profile.id.clone(),
                        ProfileRuntimeState {
                            updated_at: profile.updated_at,
                            next_update_at: profile.next_update_at,
                            last_attempt_at: profile.last_attempt_at,
                            last_update_error: profile.last_update_error.clone(),
                            revision: profile.revision,
                        },
                    )
                })
                .collect(),
        }
    }
}

pub struct AppConfigStore {
    config_path: PathBuf,
    runtime_path: PathBuf,
}

impl AppConfigStore {
    pub fn detect() -> Result<Self, String> {
        let config_dir = ensure_config_dir("")?;
        let data_dir = ensure_data_dir("")?;
        Ok(Self {
            config_path: config_dir.join("app-config.json"),
            runtime_path: data_dir.join("app-runtime.json"),
        })
    }

    pub async fn load(&self) -> Result<Option<AppConfig>, String> {
        if !fs::try_exists(&self.config_path).await.map_err(|err| {
            format!(
                "Failed to check whether application configuration exists: {}: {err}",
                self.config_path.display()
            )
        })? {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.config_path).await.map_err(|err| {
            format!(
                "Failed to read application configuration: {}: {err}",
                self.config_path.display()
            )
        })?;

        let portable: PortableConfig = serde_json::from_str(&content).map_err(|err| {
            format!(
                "Failed to parse application configuration: {}: {err}",
                self.config_path.display()
            )
        })?;

        let runtime = self.load_runtime().await?;
        Ok(Some(merge_config(portable, runtime)))
    }

    pub async fn exists(&self) -> Result<bool, String> {
        fs::try_exists(&self.config_path).await.map_err(|err| {
            format!(
                "Failed to check whether application configuration exists: {}: {err}",
                self.config_path.display()
            )
        })
    }

    pub async fn save_portable(&self, config: &AppConfig) -> Result<(), String> {
        let portable = PortableConfig::from(config);
        let content = serde_json::to_string_pretty(&portable).map_err(|err| {
            format!("Failed to serialize portable application configuration: {err}")
        })?;
        write_json(&self.config_path, "configuration", content).await
    }

    pub async fn save_runtime(&self, config: &AppConfig) -> Result<(), String> {
        let runtime = RuntimeState::from(config);
        let content = serde_json::to_string_pretty(&runtime)
            .map_err(|err| format!("Failed to serialize application runtime state: {err}"))?;
        write_json(&self.runtime_path, "runtime state", content).await
    }

    pub async fn save_all(&self, config: &AppConfig) -> Result<(), String> {
        self.save_portable(config).await?;
        self.save_runtime(config).await
    }

    async fn load_runtime(&self) -> Result<RuntimeState, String> {
        if !fs::try_exists(&self.runtime_path).await.map_err(|err| {
            format!(
                "Failed to check whether application runtime state exists: {}: {err}",
                self.runtime_path.display()
            )
        })? {
            return Ok(RuntimeState::default());
        }

        let content = fs::read_to_string(&self.runtime_path)
            .await
            .map_err(|err| {
                format!(
                    "Failed to read application runtime state: {}: {err}",
                    self.runtime_path.display()
                )
            })?;
        serde_json::from_str(&content).map_err(|err| {
            format!(
                "Failed to parse application runtime state: {}: {err}",
                self.runtime_path.display()
            )
        })
    }
}

fn merge_config(portable: PortableConfig, runtime: RuntimeState) -> AppConfig {
    let mut profiles: Vec<ProfileItem> = portable
        .profiles
        .into_iter()
        .map(ProfileItem::from)
        .collect();
    for profile in &mut profiles {
        if let Some(state) = runtime.profiles.get(&profile.id) {
            profile.updated_at = state.updated_at;
            profile.next_update_at = state.next_update_at;
            profile.last_attempt_at = state.last_attempt_at;
            profile.last_update_error = state.last_update_error.clone();
            profile.revision = state.revision;
        }
    }

    let current_profile_id = runtime
        .current_profile_id
        .filter(|id| profiles.iter().any(|profile| profile.id == *id));

    AppConfig {
        app_port: portable.app_port,
        system_proxy_enabled: portable.system_proxy_enabled,
        app_language: portable.app_language,
        profiles,
        templates: portable
            .templates
            .into_iter()
            .map(ProfileTemplate::from)
            .collect(),
        current_profile_id,
    }
}

async fn write_json(path: &PathBuf, label: &str, content: String) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|err| {
            format!(
                "Failed to create application {label} directory: {}: {err}",
                parent.display()
            )
        })?;
    }

    fs::write(path, content).await.map_err(|err| {
        format!(
            "Failed to write application {label}: {}: {err}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, PortableConfig, RuntimeState, merge_config};
    use crate::state::{ProfileItem, ProfileTemplate};

    fn profile() -> ProfileItem {
        ProfileItem {
            id: String::from("profile-one"),
            name: String::from("Main"),
            template_id: String::from("template-one"),
            inline_template: None,
            updated_at: 12,
            remotes: Vec::new(),
            hook: None,
            update_interval_hours: Some(24),
            update_cron: None,
            next_update_at: 34,
            last_attempt_at: 56,
            last_update_error: Some(String::from("old failure")),
            revision: 7,
        }
    }

    #[test]
    fn portable_projection_excludes_machine_state_and_derived_template_count() {
        let mut config = AppConfig::default();
        config.current_profile_id = Some(String::from("profile-one"));
        config.profiles.push(profile());
        config.templates.push(ProfileTemplate {
            id: String::from("template-one"),
            name: String::from("Default"),
            content: String::from("{}"),
            updated_at: 100,
            reference_count: 1,
        });

        let value = serde_json::to_value(PortableConfig::from(&config)).expect("serializes");
        assert!(value.get("current_profile_id").is_none());
        for field in [
            "next_update_at",
            "last_attempt_at",
            "last_update_error",
            "revision",
        ] {
            assert!(
                value["profiles"][0].get(field).is_none(),
                "portable state leaked {field}"
            );
        }
        assert!(value["templates"][0].get("reference_count").is_none());
        assert_eq!(value["templates"][0]["updated_at"], 100);
    }

    #[test]
    fn runtime_projection_round_trips_matching_profiles_and_drops_orphans() {
        let mut config = AppConfig::default();
        config.current_profile_id = Some(String::from("missing"));
        config.profiles.push(profile());

        let mut runtime = RuntimeState::from(&config);
        runtime.current_profile_id = Some(String::from("missing"));
        runtime.profiles.insert(
            String::from("orphan"),
            super::ProfileRuntimeState {
                updated_at: 999,
                ..Default::default()
            },
        );
        let merged = merge_config(PortableConfig::from(&config), runtime);

        assert_eq!(merged.current_profile_id, None);
        assert_eq!(merged.profiles[0].updated_at, 12);
        assert_eq!(merged.profiles[0].next_update_at, 34);
        assert_eq!(merged.profiles[0].last_attempt_at, 56);
        assert_eq!(merged.profiles[0].revision, 7);
    }
}
