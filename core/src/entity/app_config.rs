use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClashItem {
    pub name: String,
    pub subscribe_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Provider {
    #[serde(default)]
    pub clash: Vec<ClashItem>,
    #[serde(default)]
    pub singbox: Vec<ClashItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnhancementScript {
    pub inline: Option<String>,
    pub file: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub provider: Provider,
    pub secret: String,
    pub port: Option<u16>,
    #[serde(default)]
    pub enhance: Vec<EnhancementScript>,
    #[serde(default)]
    pub cache_dir: Option<String>,
}
