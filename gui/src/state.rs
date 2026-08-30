use std::time::{SystemTime, UNIX_EPOCH};

use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::config::AppConfig;

const PROFILE_ID_LENGTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum KernelStatus {
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum KernelRuntimeSource {
    None,
    CurrentInstance,
    PidFile,
}

#[derive(Debug, Clone, Serialize)]
pub struct KernelInfo {
    pub binary_path: String,
    pub installed: bool,
    pub data_dir: String,
    pub config_path: String,
    #[serde(skip_serializing)]
    pub controller_addr: String,
    #[serde(skip_serializing)]
    pub controller_secret: String,
    pub version: String,
    pub status: KernelStatus,
    pub runtime_source: KernelRuntimeSource,
    pub last_started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileTemplate {
    #[serde(
        default = "generate_profile_id",
        deserialize_with = "deserialize_profile_id"
    )]
    pub id: String,
    pub name: String,
    pub content: String,
    #[serde(default, deserialize_with = "deserialize_timestamp")]
    pub updated_at: u64,
    #[serde(default)]
    pub reference_count: usize,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ProfileRemoteFormat {
    #[default]
    Clash,
    Singbox,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteKeepFields {
    #[serde(default = "keep_outbounds")]
    pub outbounds: bool,
    #[serde(default)]
    pub inbounds: bool,
    #[serde(default)]
    pub dns: ProfileRemoteDnsKeepFields,
    #[serde(default)]
    pub route: ProfileRemoteRouteKeepFields,
    #[serde(default)]
    pub experimental: ProfileRemoteExperimentalKeepFields,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteDnsKeepFields {
    #[serde(default)]
    pub servers: bool,
    #[serde(default)]
    pub rules: bool,
    #[serde(default, rename = "final")]
    #[ts(rename = "final")]
    pub final_: bool,
    #[serde(default)]
    pub strategy: bool,
    #[serde(default)]
    pub disable_cache: bool,
    #[serde(default)]
    pub disable_expire: bool,
    #[serde(default)]
    pub independent_cache: bool,
    #[serde(default)]
    pub cache_capacity: bool,
    #[serde(default)]
    pub optimistic: ProfileRemoteDnsOptimisticKeepFields,
    #[serde(default)]
    pub timeout: bool,
    #[serde(default)]
    pub reverse_mapping: bool,
    #[serde(default)]
    pub client_subnet: bool,
    #[serde(default)]
    pub fakeip: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteDnsOptimisticKeepFields {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub timeout: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteRouteKeepFields {
    #[serde(default)]
    pub rules: bool,
    #[serde(default)]
    pub rule_set: bool,
    #[serde(default, rename = "final")]
    #[ts(rename = "final")]
    pub final_: bool,
    #[serde(default)]
    pub auto_detect_interface: bool,
    #[serde(default)]
    pub override_android_vpn: bool,
    #[serde(default)]
    pub default_interface: bool,
    #[serde(default)]
    pub default_mark: bool,
    #[serde(default)]
    pub find_process: bool,
    #[serde(default)]
    pub find_neighbor: bool,
    #[serde(default)]
    pub dhcp_lease_files: bool,
    #[serde(default)]
    pub default_http_client: bool,
    #[serde(default)]
    pub default_domain_resolver: bool,
    #[serde(default)]
    pub default_network_strategy: bool,
    #[serde(default)]
    pub default_network_type: bool,
    #[serde(default)]
    pub default_fallback_network_type: bool,
    #[serde(default)]
    pub default_fallback_delay: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteExperimentalKeepFields {
    #[serde(default)]
    pub cache_file: ProfileRemoteExperimentalCacheFileKeepFields,
    #[serde(default)]
    pub clash_api: ProfileRemoteExperimentalClashApiKeepFields,
    #[serde(default)]
    pub v2ray_api: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteExperimentalCacheFileKeepFields {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub path: bool,
    #[serde(default)]
    pub cache_id: bool,
    #[serde(default)]
    pub store_fakeip: bool,
    #[serde(default)]
    pub store_rdrc: bool,
    #[serde(default)]
    pub rdrc_timeout: bool,
    #[serde(default)]
    pub store_dns: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemoteExperimentalClashApiKeepFields {
    #[serde(default)]
    pub external_controller: bool,
    #[serde(default)]
    pub external_ui: bool,
    #[serde(default)]
    pub external_ui_download_url: bool,
    #[serde(default)]
    pub external_ui_download_detour: bool,
    #[serde(default)]
    pub secret: bool,
    #[serde(default)]
    pub default_mode: bool,
    #[serde(default)]
    pub access_control_allow_origin: bool,
    #[serde(default)]
    pub access_control_allow_private_network: bool,
    #[serde(default)]
    pub store_mode: bool,
    #[serde(default)]
    pub store_selected: bool,
    #[serde(default)]
    pub store_fakeip: bool,
    #[serde(default)]
    pub cache_file: bool,
    #[serde(default)]
    pub cache_id: bool,
}

fn keep_outbounds() -> bool {
    true
}
impl Default for ProfileRemoteKeepFields {
    fn default() -> Self {
        Self {
            outbounds: true,
            inbounds: false,
            dns: ProfileRemoteDnsKeepFields::default(),
            route: ProfileRemoteRouteKeepFields::default(),
            experimental: ProfileRemoteExperimentalKeepFields::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileHeader {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileRemote {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<ProfileHeader>,
    #[serde(default)]
    pub format: ProfileRemoteFormat,
    #[serde(default)]
    pub keep: ProfileRemoteKeepFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProfileItem {
    #[serde(
        default = "generate_profile_id",
        deserialize_with = "deserialize_profile_id"
    )]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub template_id: String,
    #[serde(default)]
    pub inline_template: Option<String>,
    #[serde(default, deserialize_with = "deserialize_timestamp")]
    pub updated_at: u64,
    #[serde(default)]
    pub remotes: Vec<ProfileRemote>,
    #[serde(default)]
    pub hook: Option<String>,
    #[serde(default)]
    pub update_interval_hours: Option<u32>,
    #[serde(default)]
    pub update_cron: Option<String>,
    #[serde(default, deserialize_with = "deserialize_timestamp")]
    pub next_update_at: u64,
    #[serde(default, deserialize_with = "deserialize_timestamp")]
    pub last_attempt_at: u64,
    #[serde(default)]
    pub last_update_error: Option<String>,
    #[serde(default)]
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppState {
    pub kernel: KernelInfo,
    pub gui_config: AppConfig,
}

impl AppState {
    pub fn load(gui_config: AppConfig) -> Self {
        Self {
            kernel: KernelInfo {
                binary_path: String::new(),
                installed: false,
                data_dir: String::new(),
                config_path: String::new(),
                controller_addr: String::from("Assigned randomly at startup"),
                controller_secret: String::from("Generated at startup"),
                version: String::from("Pending detection"),
                status: KernelStatus::Stopped,
                runtime_source: KernelRuntimeSource::None,
                last_started_at: String::from("Not started"),
            },
            gui_config,
        }
    }

    pub fn mark_kernel_running(&mut self) {
        self.kernel.status = KernelStatus::Running;
        self.kernel.last_started_at = String::from("Just now");
    }

    pub fn mark_kernel_stopped(&mut self) {
        self.kernel.status = KernelStatus::Stopped;
        self.kernel.runtime_source = KernelRuntimeSource::None;
    }

    pub fn mark_kernel_failed(&mut self) {
        self.kernel.status = KernelStatus::Failed;
        self.kernel.runtime_source = KernelRuntimeSource::None;
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn generate_profile_id() -> String {
    let mut rng = rand::rng();
    (0..PROFILE_ID_LENGTH)
        .map(|_| {
            let index = rng.random_range(0..36);
            if index < 10 {
                char::from(b'0' + index as u8)
            } else {
                char::from(b'a' + (index - 10) as u8)
            }
        })
        .collect()
}

fn deserialize_profile_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?
        .unwrap_or_default()
        .trim()
        .to_string();
    if value.is_empty() {
        Ok(generate_profile_id())
    } else {
        Ok(value)
    }
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampValue {
        Number(u64),
        Text(String),
    }

    Ok(match Option::<TimestampValue>::deserialize(deserializer)? {
        Some(TimestampValue::Number(value)) => value,
        Some(TimestampValue::Text(value)) => value.trim().parse::<u64>().unwrap_or(0),
        None => 0,
    })
}

#[cfg(test)]
mod tests {
    use super::{AppState, KernelRuntimeSource, KernelStatus, generate_profile_id};
    use crate::config::AppConfig;

    #[test]
    fn generates_eight_character_lowercase_alphanumeric_profile_ids() {
        let id = generate_profile_id();
        assert_eq!(id.len(), 8);
        assert!(
            id.bytes()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        );
    }

    #[test]
    fn explicit_stop_clears_a_failed_kernel_status() {
        let mut state = AppState::load(AppConfig::default());
        state.mark_kernel_failed();
        state.mark_kernel_stopped();

        assert_eq!(state.kernel.status, KernelStatus::Stopped);
        assert_eq!(state.kernel.runtime_source, KernelRuntimeSource::None);
    }
}
