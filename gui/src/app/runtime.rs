use std::collections::HashSet;
use std::panic::AssertUnwindSafe;
use std::str::FromStr;
use std::sync::Arc;

use futures_util::{FutureExt, future::join_all};

use crate::app::controller::AppController;
use crate::app::remote_coordinator::download_profile;
use crate::config::{AppConfig, AppConfigStore, AppLanguage};
use crate::hosts::{ProfileHost, SingBoxHost};
use crate::state::{KernelInfo, ProfileItem, ProfileRemote, ProfileRemoteFormat};
use nsb_core::{
    RemoteClashKeepFields, RemoteDnsKeepFields, RemoteDnsOptimisticKeepFields,
    RemoteExperimentalCacheFileKeepFields, RemoteExperimentalClashApiKeepFields,
    RemoteExperimentalKeepFields, RemoteFormat, RemoteKeepFields, RemoteRouteKeepFields,
    RemoteSnapshot, RemoteSource, build_config, parse_remote,
};
use tokio::sync::{Mutex, broadcast};

pub type SharedGuiRuntime = Arc<Mutex<GuiRuntime>>;

pub struct GuiRuntime {
    pub app_config_store: AppConfigStore,
    pub controller: AppController,
    pub singbox_host: SingBoxHost,
    pub profile_host: ProfileHost,
    pub updating_profiles: HashSet<String>,
    kernel_status_tx: broadcast::Sender<KernelInfo>,
}

pub(crate) fn remote_source(remote: &ProfileRemote) -> RemoteSource {
    RemoteSource {
        name: remote.name.clone(),
        url: remote.url.clone(),
        format: match remote.format {
            ProfileRemoteFormat::Clash => RemoteFormat::Clash,
            ProfileRemoteFormat::Singbox => RemoteFormat::Singbox,
            ProfileRemoteFormat::Links => RemoteFormat::Links,
        },
        keep: RemoteKeepFields {
            clash: RemoteClashKeepFields {
                proxies: remote.keep.clash.proxies,
            },
            outbounds: remote.keep.outbounds,
            inbounds: remote.keep.inbounds,
            dns: RemoteDnsKeepFields {
                servers: remote.keep.dns.servers,
                rules: remote.keep.dns.rules,
                final_: remote.keep.dns.final_,
                strategy: remote.keep.dns.strategy,
                disable_cache: remote.keep.dns.disable_cache,
                disable_expire: remote.keep.dns.disable_expire,
                independent_cache: remote.keep.dns.independent_cache,
                cache_capacity: remote.keep.dns.cache_capacity,
                optimistic: RemoteDnsOptimisticKeepFields {
                    enabled: remote.keep.dns.optimistic.enabled,
                    timeout: remote.keep.dns.optimistic.timeout,
                },
                timeout: remote.keep.dns.timeout,
                reverse_mapping: remote.keep.dns.reverse_mapping,
                client_subnet: remote.keep.dns.client_subnet,
                fakeip: remote.keep.dns.fakeip,
            },
            route: RemoteRouteKeepFields {
                rules: remote.keep.route.rules,
                rule_set: remote.keep.route.rule_set,
                final_: remote.keep.route.final_,
                auto_detect_interface: remote.keep.route.auto_detect_interface,
                override_android_vpn: remote.keep.route.override_android_vpn,
                default_interface: remote.keep.route.default_interface,
                default_mark: remote.keep.route.default_mark,
                find_process: remote.keep.route.find_process,
                find_neighbor: remote.keep.route.find_neighbor,
                dhcp_lease_files: remote.keep.route.dhcp_lease_files,
                default_http_client: remote.keep.route.default_http_client,
                default_domain_resolver: remote.keep.route.default_domain_resolver,
                default_network_strategy: remote.keep.route.default_network_strategy,
                default_network_type: remote.keep.route.default_network_type,
                default_fallback_network_type: remote.keep.route.default_fallback_network_type,
                default_fallback_delay: remote.keep.route.default_fallback_delay,
            },
            experimental: RemoteExperimentalKeepFields {
                cache_file: RemoteExperimentalCacheFileKeepFields {
                    enabled: remote.keep.experimental.cache_file.enabled,
                    path: remote.keep.experimental.cache_file.path,
                    cache_id: remote.keep.experimental.cache_file.cache_id,
                    store_fakeip: remote.keep.experimental.cache_file.store_fakeip,
                    store_rdrc: remote.keep.experimental.cache_file.store_rdrc,
                    rdrc_timeout: remote.keep.experimental.cache_file.rdrc_timeout,
                    store_dns: remote.keep.experimental.cache_file.store_dns,
                },
                clash_api: RemoteExperimentalClashApiKeepFields {
                    external_controller: remote.keep.experimental.clash_api.external_controller,
                    external_ui: remote.keep.experimental.clash_api.external_ui,
                    external_ui_download_url: remote
                        .keep
                        .experimental
                        .clash_api
                        .external_ui_download_url,
                    external_ui_download_detour: remote
                        .keep
                        .experimental
                        .clash_api
                        .external_ui_download_detour,
                    secret: remote.keep.experimental.clash_api.secret,
                    default_mode: remote.keep.experimental.clash_api.default_mode,
                    access_control_allow_origin: remote
                        .keep
                        .experimental
                        .clash_api
                        .access_control_allow_origin,
                    access_control_allow_private_network: remote
                        .keep
                        .experimental
                        .clash_api
                        .access_control_allow_private_network,
                    store_mode: remote.keep.experimental.clash_api.store_mode,
                    store_selected: remote.keep.experimental.clash_api.store_selected,
                    store_fakeip: remote.keep.experimental.clash_api.store_fakeip,
                    cache_file: remote.keep.experimental.clash_api.cache_file,
                    cache_id: remote.keep.experimental.clash_api.cache_id,
                },
                v2ray_api: remote.keep.experimental.v2ray_api,
            },
        },
    }
}

#[derive(Debug)]
pub(crate) struct RemoteCollection {
    pub snapshots: Vec<RemoteSnapshot>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug)]
enum CachedRemoteSnapshot {
    Missing,
    ReadError(String),
    ParseError(String),
    Parsed(RemoteSnapshot),
}

/// Collects all usable snapshots for a Profile while retaining the configured
/// remote count for the caller. A refresh downloads each entry; kernel start
/// passes `false` so this path only reads the raw cache.
pub(crate) async fn collect_remote_snapshots(
    profile: &ProfileItem,
    profile_host: &ProfileHost,
    download_remotes: bool,
) -> Result<RemoteCollection, String> {
    let multi_remote = profile.remotes.len() > 1;
    let mut snapshots = Vec::new();
    let mut diagnostics = Vec::new();

    let results = join_all(profile.remotes.iter().map(|remote| {
        collect_remote_snapshot(profile_host.clone(), remote, download_remotes, multi_remote)
    }))
    .await;

    for result in results {
        let result = result?;
        if let Some(snapshot) = result.snapshot {
            for warning in &snapshot.warnings {
                log::warn!("{warning}");
                diagnostics.push(format!(
                    "Remote parser warning: {}",
                    safe_remote_error(warning)
                ));
            }
            snapshots.push(snapshot);
        }
        if let Some(diagnostic) = result.diagnostic {
            diagnostics.push(diagnostic);
        }
    }

    Ok(RemoteCollection {
        snapshots,
        diagnostics,
    })
}

struct RemoteCollectionResult {
    snapshot: Option<RemoteSnapshot>,
    diagnostic: Option<String>,
}

async fn collect_remote_snapshot(
    profile_host: ProfileHost,
    remote: &ProfileRemote,
    download_remotes: bool,
    multi_remote: bool,
) -> Result<RemoteCollectionResult, String> {
    let downloaded = if download_remotes {
        Some(
            match AssertUnwindSafe(download_profile(&remote.url, &remote.headers))
                .catch_unwind()
                .await
            {
                Ok(result) => result,
                Err(_) => Err(String::from("Remote download worker panicked.")),
            },
        )
    } else {
        None
    };

    if let Some(downloaded) = downloaded {
        match downloaded {
                Ok(content) => match parse_remote(&remote_source(remote), &content, multi_remote) {
                    Ok(snapshot) => {
                        profile_host.save_remote_raw(&remote.url, &content).await?;
                        return Ok(RemoteCollectionResult {
                            snapshot: Some(snapshot),
                            diagnostic: None,
                        });
                    }
                    Err(parse_error) => {
                        let cache = read_cached_snapshot(
                            &profile_host,
                            remote,
                            multi_remote,
                        )
                        .await;
                        match cache {
                            CachedRemoteSnapshot::Parsed(snapshot) => {
                                return Ok(RemoteCollectionResult {
                                    snapshot: Some(snapshot),
                                    diagnostic: Some(format!(
                                        "Remote '{}' returned invalid content; using cached content. Reason: {}",
                                        safe_remote_name(&remote.name),
                                        safe_remote_error(&parse_error),
                                    )),
                                });
                            }
                            CachedRemoteSnapshot::Missing => return Ok(RemoteCollectionResult {
                                snapshot: None,
                                diagnostic: Some(format!(
                                    "Remote '{}' returned invalid content; skipped because no cached content is available. Reason: {}",
                                    safe_remote_name(&remote.name),
                                    safe_remote_error(&parse_error),
                                )),
                            }),
                            CachedRemoteSnapshot::ReadError(cache_error) => return Ok(RemoteCollectionResult {
                                snapshot: None,
                                diagnostic: Some(format!(
                                    "Remote '{}' returned invalid content; skipped because cached content could not be read. Reason: {}; cache error: {}",
                                    safe_remote_name(&remote.name),
                                    safe_remote_error(&parse_error),
                                    safe_remote_error(&cache_error),
                                )),
                            }),
                            CachedRemoteSnapshot::ParseError(cache_error) => return Ok(RemoteCollectionResult {
                                snapshot: None,
                                diagnostic: Some(format!(
                                    "Remote '{}' returned invalid content; skipped because cached content is also invalid. Reason: {}; cache error: {}",
                                    safe_remote_name(&remote.name),
                                    safe_remote_error(&parse_error),
                                    safe_remote_error(&cache_error),
                                )),
                            }),
                        }
                    }
                },
                Err(download_error) => {
                    let cache = read_cached_snapshot(
                        &profile_host,
                        remote,
                        multi_remote,
                    )
                    .await;
                    match cache {
                        CachedRemoteSnapshot::Parsed(snapshot) => {
                            return Ok(RemoteCollectionResult {
                                snapshot: Some(snapshot),
                                diagnostic: Some(format!(
                                    "Remote '{}' download failed; using cached content. Reason: {}",
                                    safe_remote_name(&remote.name),
                                    safe_remote_error(&download_error),
                                )),
                            });
                        }
                        CachedRemoteSnapshot::Missing => return Ok(RemoteCollectionResult {
                            snapshot: None,
                            diagnostic: Some(format!(
                                "Remote '{}' download failed; skipped because no cached content is available. Reason: {}",
                                safe_remote_name(&remote.name),
                                safe_remote_error(&download_error),
                            )),
                        }),
                        CachedRemoteSnapshot::ReadError(cache_error) => return Ok(RemoteCollectionResult {
                            snapshot: None,
                            diagnostic: Some(format!(
                                "Remote '{}' download failed; skipped because cached content could not be read. Reason: {}; cache error: {}",
                                safe_remote_name(&remote.name),
                                safe_remote_error(&download_error),
                                safe_remote_error(&cache_error),
                            )),
                        }),
                        CachedRemoteSnapshot::ParseError(cache_error) => return Ok(RemoteCollectionResult {
                            snapshot: None,
                            diagnostic: Some(format!(
                                "Remote '{}' download failed; skipped because cached content is invalid. Reason: {}; cache error: {}",
                                safe_remote_name(&remote.name),
                                safe_remote_error(&download_error),
                                safe_remote_error(&cache_error),
                            )),
                        }),
                    }
                }
            }
        }

    match read_cached_snapshot(&profile_host, remote, multi_remote).await {
        CachedRemoteSnapshot::Parsed(snapshot) => Ok(RemoteCollectionResult {
            snapshot: Some(snapshot),
            diagnostic: None,
        }),
        CachedRemoteSnapshot::Missing => Ok(RemoteCollectionResult {
            snapshot: None,
            diagnostic: Some(format!(
                "Remote '{}' has no cached content; skipped.",
                safe_remote_name(&remote.name)
            )),
        }),
        CachedRemoteSnapshot::ReadError(error) => Ok(RemoteCollectionResult {
            snapshot: None,
            diagnostic: Some(format!(
                "Remote '{}' cached content could not be read; skipped. Reason: {}",
                safe_remote_name(&remote.name),
                safe_remote_error(&error)
            )),
        }),
        CachedRemoteSnapshot::ParseError(error) => Ok(RemoteCollectionResult {
            snapshot: None,
            diagnostic: Some(format!(
                "Remote '{}' cached content is invalid; skipped. Reason: {}",
                safe_remote_name(&remote.name),
                safe_remote_error(&error)
            )),
        }),
    }
}

async fn read_cached_snapshot(
    profile_host: &ProfileHost,
    remote: &ProfileRemote,
    multi_remote: bool,
) -> CachedRemoteSnapshot {
    let content = match profile_host.read_remote_raw(&remote.url).await {
        Ok(Some(content)) => content,
        Ok(None) => return CachedRemoteSnapshot::Missing,
        Err(error) => return CachedRemoteSnapshot::ReadError(error),
    };

    match parse_remote(&remote_source(remote), &content, multi_remote) {
        Ok(snapshot) => CachedRemoteSnapshot::Parsed(snapshot),
        Err(error) => CachedRemoteSnapshot::ParseError(error),
    }
}

pub(crate) fn safe_remote_name(name: &str) -> String {
    let name: String = name
        .chars()
        .filter(|character| !character.is_control())
        .take(80)
        .collect();
    if name.trim().is_empty() {
        String::from("unnamed")
    } else {
        safe_remote_error(&name)
            .chars()
            .take(80)
            .collect()
    }
}

pub(crate) fn safe_remote_error(error: &str) -> String {
    let mut sanitized = error
        .split_whitespace()
        .map(|token| {
            if token.contains("://") {
                String::from("[redacted URL]")
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    for marker in [
        "token=",
        "access_token=",
        "api_key=",
        "password=",
        "secret=",
        "authorization=",
        "authorization:",
        "cookie:",
        "x-api-key:",
    ] {
        let lower = sanitized.to_ascii_lowercase();
        let Some(start) = lower.find(marker) else {
            continue;
        };
        let value_start = start + marker.len();
        let value_end = sanitized[value_start..]
            .find(char::is_whitespace)
            .map(|offset| value_start + offset)
            .unwrap_or(sanitized.len());
        sanitized.replace_range(value_start..value_end, "[redacted]");
    }

    sanitized = sanitized.chars().take(240).collect();
    if sanitized.is_empty() {
        String::from("unspecified error")
    } else {
        sanitized
    }
}

impl GuiRuntime {
    pub async fn detect() -> Result<Self, String> {
        let log_path = crate::logger::default_log_path()?;
        crate::logger::set_log_file(&log_path);

        let mut singbox_host = SingBoxHost::detect()?;
        let profile_host = ProfileHost::detect()?;

        let app_config_store = AppConfigStore::detect()?;
        let app_config = app_config_store
            .load()
            .await?
            .unwrap_or_else(AppConfig::default);
        let mut controller = AppController::new(app_config);
        let normalized_names = controller.normalize_unique_names();
        controller
            .bootstrap_runtime(&mut singbox_host, &profile_host)
            .await;
        if normalized_names || !app_config_store.exists().await? {
            app_config_store
                .save_portable(&controller.state.gui_config)
                .await?;
        }

        let (kernel_status_tx, _) = broadcast::channel(16);
        Ok(Self {
            app_config_store,
            controller,
            singbox_host,
            profile_host,
            updating_profiles: HashSet::new(),
            kernel_status_tx,
        })
    }

    pub async fn sync_runtime(&mut self) {
        self.controller.sync_runtime(&mut self.singbox_host).await;
    }

    pub async fn toggle_kernel(&mut self) -> Result<bool, String> {
        let result = self
            .controller
            .toggle_kernel(&mut self.singbox_host, &self.profile_host)
            .await;
        self.publish_kernel_status();
        result
    }

    pub async fn restart_kernel(&mut self) -> Result<(), String> {
        let result = self
            .controller
            .restart_kernel(&mut self.singbox_host, &self.profile_host)
            .await;
        self.publish_kernel_status();
        result
    }

    pub async fn replace_kernel_binary(&mut self, bytes: &[u8]) -> Result<(), String> {
        let result = async {
            let was_running = self.singbox_host.running_pid().await?.is_some();
            if was_running {
                self.controller.stop_kernel(&mut self.singbox_host).await?;
            }
            self.singbox_host.install_binary(bytes).await?;
            if was_running {
                self.controller
                    .start_kernel(&mut self.singbox_host, &self.profile_host)
                    .await?;
            }
            Ok(())
        }
        .await;
        self.publish_kernel_status();
        result
    }

    pub async fn create_profile(
        &mut self,
        name: String,
        update_interval_hours: Option<u32>,
        update_cron: Option<String>,
    ) -> Result<(), String> {
        self.controller
            .create_profile(
                name,
                update_interval_hours,
                update_cron,
                &self.app_config_store,
            )
            .await
    }

    pub async fn update_profile(
        &mut self,
        id: String,
        name: String,
        update_interval_hours: Option<u32>,
        update_cron: Option<String>,
    ) -> Result<(), String> {
        self.controller
            .update_profile(
                id,
                name,
                update_interval_hours,
                update_cron,
                &self.app_config_store,
            )
            .await
    }

    pub async fn delete_profile(&mut self, id: String) -> Result<(), String> {
        let previous_status = self.controller.state.kernel.status;
        let result = self
            .controller
            .delete_profile(
                id,
                &mut self.singbox_host,
                &self.profile_host,
                &self.app_config_store,
            )
            .await;
        if self.controller.state.kernel.status != previous_status {
            self.publish_kernel_status();
        }
        result
    }

    pub async fn read_profile_runtime(&self, id: &str) -> Result<String, String> {
        self.controller
            .read_profile_runtime(id, &self.profile_host)
            .await
    }

    pub async fn save_profile_runtime(&mut self, id: &str, content: String) -> Result<(), String> {
        self.controller
            .save_profile_runtime(id, content, &self.profile_host, &self.app_config_store)
            .await
    }

    pub async fn activate_profile(&mut self, id: String) -> Result<(), String> {
        let result = self
            .controller
            .activate_profile(
                id,
                &mut self.singbox_host,
                &self.profile_host,
                &self.app_config_store,
            )
            .await;
        self.publish_kernel_status();
        result
    }

    pub async fn save_runtime_settings(
        &mut self,
        app_port: u16,
        system_proxy_enabled: bool,
    ) -> Result<(), String> {
        self.controller
            .save_runtime_settings(
                app_port,
                system_proxy_enabled,
                &self.profile_host,
                &self.app_config_store,
            )
            .await?;

        Ok(())
    }

    pub async fn save_app_language(&mut self, app_language: AppLanguage) -> Result<(), String> {
        self.controller
            .save_app_language(app_language, &self.app_config_store)
            .await
    }

    pub async fn auto_start_kernel_if_needed(&mut self) -> Result<bool, String> {
        let result = self
            .controller
            .auto_start_kernel_if_needed(&mut self.singbox_host, &self.profile_host)
            .await;
        self.publish_kernel_status();
        result
    }

    pub async fn shutdown_kernel_on_exit(&mut self) -> Result<(), String> {
        let result = self
            .controller
            .shutdown_kernel_on_exit(&mut self.singbox_host)
            .await;
        self.publish_kernel_status();
        result
    }

    pub fn subscribe_kernel_status(&self) -> broadcast::Receiver<KernelInfo> {
        self.kernel_status_tx.subscribe()
    }

    fn publish_kernel_status(&self) {
        let _ = self
            .kernel_status_tx
            .send(self.controller.state.kernel.clone());
    }
}

pub async fn update_profile_runtime(
    runtime: SharedGuiRuntime,
    profile_id: String,
    restart_current_kernel: bool,
) -> Result<(), String> {
    let profile = {
        let mut guard = runtime.lock().await;
        if !guard.updating_profiles.insert(profile_id.clone()) {
            return Err(String::from("Profile update is already in progress."));
        }

        match guard
            .controller
            .state
            .gui_config
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .cloned()
        {
            Some(profile) => profile,
            None => {
                guard.updating_profiles.remove(&profile_id);
                return Err(String::from("Profile to update was not found."));
            }
        }
    };

    let profile_host = {
        let guard = runtime.lock().await;
        guard.profile_host.clone()
    };
    let result = async {
        let template = if let Some(content) = profile.inline_template.clone() {
            content
        } else {
            let guard = runtime.lock().await;
            guard
                .controller
                .state
                .gui_config
                .templates
                .iter()
                .find(|item| item.id == profile.template_id)
                .map(|item| item.content.clone())
                .ok_or_else(|| String::from("Profile references a missing Template."))?
        };
        let collection = collect_remote_snapshots(&profile, &profile_host, true).await?;
        let diagnostics = collection.diagnostics;
        build_config(
            &template,
            collection.snapshots,
            profile.hook.as_deref(),
            profile.remotes.len() > 1,
        )
        .map(|content| (content, diagnostics))
        .map_err(
                |error| {
                    log::error!(
                        "Failed to generate Profile runtime configuration: profile_id={} remotes={} error={error}",
                        profile.id,
                        profile.remotes.len()
                    );
                    error
                },
            )
    }
    .await;

    let mut guard = runtime.lock().await;
    guard.updating_profiles.remove(&profile_id);
    let now = crate::state::current_timestamp();
    let Some(index) = guard
        .controller
        .state
        .gui_config
        .profiles
        .iter()
        .position(|item| item.id == profile.id && item.revision == profile.revision)
    else {
        return Ok(());
    };

    let next_update_at = next_update_at(&profile, now);
    let is_current = guard
        .controller
        .state
        .gui_config
        .current_profile_id
        .as_deref()
        == Some(profile.id.as_str());

    match result {
        Ok((content, diagnostics)) => {
            guard
                .profile_host
                .save_runtime(&profile.id, &content)
                .await?;
            {
                let target = &mut guard.controller.state.gui_config.profiles[index];
                target.updated_at = now;
                target.last_attempt_at = now;
                target.last_update_error = (!diagnostics.is_empty()).then(|| diagnostics.join("\n"));
                target.next_update_at = next_update_at;
            }
            guard
                .app_config_store
                .save_runtime(&guard.controller.state.gui_config)
                .await?;

            if restart_current_kernel && is_current {
                // Refresh is explicitly asked to apply the active Profile to the
                // core. The status may be stale when the core exited between UI
                // snapshots. The shared restart wrapper synchronizes it before
                // applying the newly persisted runtime configuration and publishes
                // only the final kernel snapshot.
                guard.restart_kernel().await?;
            }
            Ok(())
        }
        Err(error) => {
            {
                let target = &mut guard.controller.state.gui_config.profiles[index];
                target.last_attempt_at = now;
                target.last_update_error = Some(error.clone());
                target.next_update_at = next_update_at;
            }
            guard
                .app_config_store
                .save_runtime(&guard.controller.state.gui_config)
                .await?;
            Err(error)
        }
    }
}

pub async fn due_profile_ids(runtime: SharedGuiRuntime) -> Vec<String> {
    let guard = runtime.lock().await;
    let now = crate::state::current_timestamp();
    guard
        .controller
        .state
        .gui_config
        .profiles
        .iter()
        .filter(|profile| {
            profile.next_update_at > 0
                && profile.next_update_at <= now
                && !guard.updating_profiles.contains(&profile.id)
        })
        .map(|profile| profile.id.clone())
        .collect()
}

fn next_update_at(profile: &crate::state::ProfileItem, now: u64) -> u64 {
    if let Some(hours) = profile.update_interval_hours {
        return now.saturating_add(u64::from(hours).saturating_mul(60 * 60));
    }

    let Some(expression) = profile.update_cron.as_deref() else {
        return 0;
    };
    let expression = format!("0 {} *", expression.trim());
    let Ok(schedule) = cron::Schedule::from_str(&expression) else {
        return 0;
    };
    schedule
        .after(&chrono::Local::now())
        .next()
        .and_then(|time| u64::try_from(time.timestamp()).ok())
        .unwrap_or(0)
}
