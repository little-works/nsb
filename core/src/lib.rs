pub mod entity;
pub mod subscription;

pub use entity::*;
pub use subscription::{
    RemoteDnsKeepFields, RemoteDnsOptimisticKeepFields, RemoteExperimentalCacheFileKeepFields,
    RemoteExperimentalClashApiKeepFields, RemoteExperimentalKeepFields, RemoteFormat,
    RemoteKeepFields, RemoteRouteKeepFields, RemoteSnapshot, RemoteSource, build_config,
    default_template, parse_remote, run_finalize_hook,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Clash,
    SingBox,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSource {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub sources: Vec<ProfileSource>,
    pub default_format: OutputFormat,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8080".to_string(),
            sources: Vec::new(),
            default_format: OutputFormat::SingBox,
        }
    }
}
