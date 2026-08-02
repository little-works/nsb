use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ClashWsOpts {
    pub path: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ClashProxy {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub server: Option<String>,
    pub port: usize,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "alterId")]
    pub alert_id: Option<usize>,
    pub cipher: Option<String>,
    pub udp: Option<bool>,
    pub uuid: Option<String>,
    pub sni: Option<String>,
    pub ws_opts: Option<ClashWsOpts>,
    pub skip_cert_verify: Option<bool>,
    pub network: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct RuleProvider {
    #[serde(rename = "type")]
    pub _type: String,
    pub behavior: String,
    pub url: String,
    pub path: Option<String>,
    pub interval: Option<usize>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub proxies: Vec<String>,
    pub interval: Option<usize>,
    pub url: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ClashConfig {
    #[serde(default)]
    pub port: usize,
    #[serde(default)]
    pub allow_lan: bool,
    #[serde(default)]
    pub proxies: Vec<ClashProxy>,
    #[serde(default)]
    pub proxy_groups: Vec<ProxyGroup>,
    #[serde(default)]
    pub rule_providers: HashMap<String, RuleProvider>,
    #[serde(default)]
    pub rules: Vec<String>,
}
