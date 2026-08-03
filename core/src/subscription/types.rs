use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSource {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSnapshot {
    pub name: String,
    pub url: String,
    pub proxy_nodes: Vec<Value>,
    pub proxy_groups: Vec<Value>,
    pub route_rules: Vec<Value>,
    pub warnings: Vec<String>,
    pub updated_at: u64,
}
