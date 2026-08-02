use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct V2RayTransport {
    #[serde(rename = "type")]
    pub type_: String,
    pub path: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Utls {
    pub enabled: bool,
    pub fingerprint: String,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct OutboundTls {
    pub enabled: bool,
    pub disable_sni: Option<bool>,
    pub server_name: Option<String>,
    pub insecure: Option<bool>,
    pub utls: Option<Utls>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Outbound {
    pub tag: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub server: Option<String>,
    pub server_port: Option<usize>,
    pub alter_id: Option<usize>,
    pub uuid: Option<String>,
    pub security: Option<String>,
    pub outbounds: Option<Vec<String>>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub transport: Option<V2RayTransport>,
    pub tls: Option<OutboundTls>,
    pub url: Option<String>,
    pub interval: Option<String>,
    pub tolerance: Option<usize>,
    pub interrupt_exist_connections: Option<bool>,
    pub domain_resolver: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Inbound {
    #[serde(rename = "type")]
    pub type_: String,
    pub tag: String,
    pub listen: String,
    pub listen_port: usize,
    pub tcp_fast_open: Option<bool>,
    pub tcp_multi_path: Option<bool>,
    pub udp_fragment: Option<bool>,
    pub domain_strategy: Option<String>,
    pub users: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RouteRule {
    pub action: Option<String>,
    pub outbound: Option<String>,
    pub server: Option<String>,
    pub protocol: Option<String>,
    pub network: Option<String>,
    pub clash_mode: Option<String>,
    pub domain_suffix: Option<Vec<String>>,
    pub rule_set: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RuleSet {
    pub tag: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    pub format: String,
    pub download_detour: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DefaultDomainResolver {
    pub server: String,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Route {
    #[serde(rename = "final")]
    pub final_: String,
    pub rules: Vec<RouteRule>,
    pub rule_set: Vec<RuleSet>,
    pub auto_detect_interface: bool,
    pub default_domain_resolver: Option<DefaultDomainResolver>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DnsServer {
    pub tag: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub address: Option<String>,
    pub server: Option<String>,
    pub server_port: Option<usize>,
    pub path: Option<String>,
    pub detour: Option<String>,
    pub domain_resolver: Option<String>,
    pub inet4_range: Option<String>,
    pub inet6_range: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DnsRule {
    pub action: Option<String>,
    pub server: String,
    pub domain_suffix: Option<Vec<String>>,
    pub rule_set: Option<Vec<String>>,
    pub clash_mode: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DnsFakeIp {
    pub enabled: bool,
    pub inet4_range: Option<String>,
    pub inet6_range: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Dns {
    #[serde(rename = "final")]
    pub final_: Option<String>,
    pub servers: Vec<DnsServer>,
    pub rules: Vec<DnsRule>,
    pub independent_cache: Option<bool>,
    pub disable_cache: Option<bool>,
    pub disable_expire: Option<bool>,
    pub fakeip: Option<DnsFakeIp>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ClashApi {
    pub external_controller: Option<String>,
    pub external_ui: Option<String>,
    pub external_ui_download_url: Option<String>,
    pub external_ui_download_detour: Option<String>,
    pub secret: Option<String>,
    pub default_mode: Option<String>,
    pub access_control_allow_origin: Option<Vec<String>>,
    pub access_control_allow_private_network: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct CacheFile {
    pub enabled: bool,
    pub path: Option<String>,
    pub cache_id: Option<String>,
    pub store_fakeip: Option<bool>,
    pub rdrc_timeout: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Experimental {
    pub clash_api: Option<ClashApi>,
    pub cache_file: Option<CacheFile>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Log {
    pub disabled: Option<bool>,
    pub level: Option<String>,
    pub output: Option<String>,
    pub timestamp: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SingBoxConfig {
    pub log: Option<Log>,
    #[serde(default)]
    pub outbounds: Vec<Outbound>,
    #[serde(default)]
    pub route: Route,
    pub dns: Option<Dns>,
    #[serde(default)]
    pub inbounds: Vec<Inbound>,
    pub experimental: Option<Experimental>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
