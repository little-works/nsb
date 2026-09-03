use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct V2RayTransport {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub path: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Utls {
    pub enabled: Option<bool>,
    pub fingerprint: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct OutboundTls {
    pub enabled: Option<bool>,
    pub disable_sni: Option<bool>,
    pub server_name: Option<String>,
    pub insecure: Option<bool>,
    pub utls: Option<Utls>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Outbound {
    pub tag: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
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
    pub type_: Option<String>,
    pub tag: Option<String>,
    pub listen: Option<String>,
    pub listen_port: Option<usize>,
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
    pub tag: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub rules: Option<Vec<Value>>,
    pub path: Option<String>,
    pub url: Option<String>,
    pub format: Option<String>,
    pub initial_path: Option<String>,
    pub http_client: Option<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DefaultDomainResolver {
    pub server: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Route {
    #[serde(rename = "final")]
    pub final_: Option<String>,
    pub rules: Option<Vec<RouteRule>>,
    pub rule_set: Option<Vec<RuleSet>>,
    pub auto_detect_interface: Option<bool>,
    pub default_domain_resolver: Option<DefaultDomainResolver>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DnsServer {
    pub tag: Option<String>,
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
    pub server: Option<String>,
    pub domain_suffix: Option<Vec<String>>,
    pub rule_set: Option<Vec<String>>,
    pub clash_mode: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DnsFakeIp {
    pub enabled: Option<bool>,
    pub inet4_range: Option<String>,
    pub inet6_range: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Dns {
    #[serde(rename = "final")]
    pub final_: Option<String>,
    pub servers: Option<Vec<DnsServer>>,
    pub rules: Option<Vec<DnsRule>>,
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
    pub enabled: Option<bool>,
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
    pub outbounds: Option<Vec<Outbound>>,
    pub route: Option<Route>,
    pub dns: Option<Dns>,
    pub inbounds: Option<Vec<Inbound>>,
    pub experimental: Option<Experimental>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::SingBoxConfig;

    #[test]
    fn parses_partial_template_with_inline_rule_set() {
        let config: SingBoxConfig = serde_json::from_str(
            r#"{
                "route": {
                    "rule_set": [{
                        "type": "inline",
                        "tag": "private",
                        "rules": [{"domain_suffix": ["example.test"]}]
                    }]
                }
            }"#,
        )
        .unwrap();

        let rule_set = config
            .route
            .as_ref()
            .and_then(|route| route.rule_set.as_ref())
            .and_then(|rule_sets| rule_sets.first())
            .unwrap();
        assert_eq!(rule_set.type_.as_deref(), Some("inline"));
        assert_eq!(rule_set.rules.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn parses_an_empty_configuration_fragment() {
        let config: SingBoxConfig = serde_json::from_str("{}").unwrap();

        assert!(config.inbounds.is_none());
        assert!(config.outbounds.is_none());
        assert!(config.route.is_none());
    }
}
