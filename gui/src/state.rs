use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::config::AppConfig;

static PROFILE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum KernelStatus {
    Running,
    Stopped,
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
#[serde(rename_all = "UPPERCASE")]
#[ts(export)]
pub enum ProfileKind {
    #[ts(rename = "URL")]
    Url,
    #[ts(rename = "FILE")]
    File,
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
    pub kind: ProfileKind,
    pub url: String,
    #[serde(default, deserialize_with = "deserialize_timestamp")]
    pub updated_at: u64,
    #[serde(default)]
    pub headers: Vec<ProfileHeader>,
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

impl ProfileItem {
    pub fn source_kind(source: &str) -> ProfileKind {
        if source.starts_with("http://") || source.starts_with("https://") {
            ProfileKind::Url
        } else {
            ProfileKind::File
        }
    }

    pub fn normalized_remotes(&self) -> Vec<ProfileRemote> {
        if !self.remotes.is_empty() {
            return self.remotes.clone();
        }
        if self.url.trim().is_empty() {
            return Vec::new();
        }
        vec![ProfileRemote {
            name: String::from("Default"),
            url: self.url.clone(),
            headers: self.headers.clone(),
        }]
    }
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
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn generate_profile_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = PROFILE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("profile-{nanos:032x}-{counter:08x}")
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
