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
    pub outbounds: bool,
    #[serde(default)]
    pub inbounds: bool,
    #[serde(default)]
    pub dns: RemoteDnsKeepFields,
    #[serde(default)]
    pub route: RemoteRouteKeepFields,
    #[serde(default)]
    pub experimental: RemoteExperimentalKeepFields,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteDnsKeepFields {
    #[serde(default)]
    pub servers: bool,
    #[serde(default)]
    pub rules: bool,
    #[serde(default, rename = "final")]
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
    pub optimistic: RemoteDnsOptimisticKeepFields,
    #[serde(default)]
    pub timeout: bool,
    #[serde(default)]
    pub reverse_mapping: bool,
    #[serde(default)]
    pub client_subnet: bool,
    #[serde(default)]
    pub fakeip: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteDnsOptimisticKeepFields {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub timeout: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteRouteKeepFields {
    #[serde(default)]
    pub rules: bool,
    #[serde(default)]
    pub rule_set: bool,
    #[serde(default, rename = "final")]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteExperimentalKeepFields {
    #[serde(default)]
    pub cache_file: RemoteExperimentalCacheFileKeepFields,
    #[serde(default)]
    pub clash_api: RemoteExperimentalClashApiKeepFields,
    #[serde(default)]
    pub v2ray_api: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteExperimentalCacheFileKeepFields {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteExperimentalClashApiKeepFields {
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

fn keep_true() -> bool {
    true
}

impl Default for RemoteKeepFields {
    fn default() -> Self {
        Self {
            outbounds: true,
            inbounds: false,
            dns: RemoteDnsKeepFields::default(),
            route: RemoteRouteKeepFields::default(),
            experimental: RemoteExperimentalKeepFields::default(),
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
    #[serde(default)]
    pub dns: Option<Value>,
    #[serde(default)]
    pub inbounds: Vec<Value>,
    #[serde(default)]
    pub route: Option<Value>,
    #[serde(default)]
    pub experimental: Option<Value>,
    pub warnings: Vec<String>,
    pub updated_at: u64,
}
