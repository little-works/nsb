mod builder;
mod hooks;
mod parsing;
mod types;

#[cfg(test)]
mod tests;

pub use builder::build_config;
pub use hooks::run_finalize_hook;
pub use parsing::parse_remote;
pub use types::{RemoteSnapshot, RemoteSource};
