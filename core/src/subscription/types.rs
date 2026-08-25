use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteFormat {
    #[default]
    Clash,
    Singbox,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteKeepFields {
    #[serde(default = "keep_true")]
    pub nodes: bool,
    #[serde(default)]
    pub groups: bool,
    #[serde(default)]
    pub route_final: bool,
    #[serde(default)]
    pub route_rules: bool,
}

fn keep_true() -> bool {
    true
}

impl Default for RemoteKeepFields {
    fn default() -> Self {
        Self {
            nodes: true,
            groups: false,
            route_final: false,
            route_rules: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSource {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub format: RemoteFormat,
    #[serde(default)]
    pub keep: RemoteKeepFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSnapshot {
    pub name: String,
    pub url: String,
    pub proxy_nodes: Vec<Value>,
    pub proxy_groups: Vec<Value>,
    pub route_rules: Vec<Value>,
    #[serde(default)]
    pub route_final: Option<String>,
    pub warnings: Vec<String>,
    pub updated_at: u64,
}
