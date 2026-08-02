use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;
use ts_rs::TS;

use crate::state::ProfileItem;
use crate::utils::path::ensure_data_dir;

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
    pub mixed_port: u16,
    pub allow_lan: bool,
    #[serde(default = "default_app_port")]
    pub app_port: u16,
    #[serde(default)]
    pub system_proxy_enabled: bool,
    #[serde(default)]
    pub app_language: AppLanguage,
    #[serde(default)]
    pub profiles: Vec<ProfileItem>,
    #[serde(default)]
    pub current_profile_id: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mixed_port: 7990,
            allow_lan: false,
            app_port: default_app_port(),
            system_proxy_enabled: false,
            app_language: AppLanguage::default(),
            profiles: Vec::new(),
            current_profile_id: None,
        }
    }
}

fn default_app_port() -> u16 {
    8787
}

pub struct AppConfigStore {
    config_path: PathBuf,
}

impl AppConfigStore {
    pub fn detect() -> Result<Self, String> {
        let data_dir = ensure_data_dir("")?;
        Ok(Self {
            config_path: data_dir.join("app-config.json"),
        })
    }

    pub async fn load(&self) -> Result<Option<AppConfig>, String> {
        if !self.exists().await? {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.config_path).await.map_err(|err| {
            format!(
                "Failed to read application configuration: {}: {err}",
                self.config_path.display()
            )
        })?;

        let config = serde_json::from_str(&content).map_err(|err| {
            format!(
                "Failed to parse application configuration: {}: {err}",
                self.config_path.display()
            )
        })?;

        Ok(Some(config))
    }

    pub async fn exists(&self) -> Result<bool, String> {
        fs::try_exists(&self.config_path).await.map_err(|err| {
            format!(
                "Failed to check whether application configuration exists: {}: {err}",
                self.config_path.display()
            )
        })
    }

    pub async fn save(&self, config: &AppConfig) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                format!(
                    "Failed to create configuration directory: {}: {err}",
                    parent.display()
                )
            })?;
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|err| format!("Failed to serialize application configuration: {err}"))?;
        fs::write(&self.config_path, content).await.map_err(|err| {
            format!(
                "Failed to write application configuration: {}: {err}",
                self.config_path.display()
            )
        })
    }
}
