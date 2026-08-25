use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use crate::app::controller::AppController;
use crate::config::{AppConfig, AppConfigStore, AppLanguage};
use crate::hosts::{ProfileHost, SingBoxHost};
use crate::state::KernelStatus;
use nsb_core::{RemoteFormat, RemoteKeepFields, RemoteSource, build_config, parse_remote};
use tokio::sync::Mutex;

pub type SharedGuiRuntime = Arc<Mutex<GuiRuntime>>;

pub struct GuiRuntime {
    pub app_config_store: AppConfigStore,
    pub controller: AppController,
    pub singbox_host: SingBoxHost,
    pub profile_host: ProfileHost,
    pub updating_profiles: HashSet<String>,
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
        controller
            .bootstrap_runtime(&mut singbox_host, &profile_host)
            .await;
        if !app_config_store.exists().await? {
            app_config_store.save(&controller.state.gui_config).await?;
        }

        Ok(Self {
            app_config_store,
            controller,
            singbox_host,
            profile_host,
            updating_profiles: HashSet::new(),
        })
    }

    pub async fn sync_runtime(&mut self) {
        self.controller.sync_runtime(&mut self.singbox_host).await;
    }

    pub async fn toggle_kernel(&mut self) -> Result<bool, String> {
        self.controller
            .toggle_kernel(
                &mut self.singbox_host,
                &self.profile_host,
                &self.app_config_store,
            )
            .await
    }

    pub async fn replace_kernel_binary(&mut self, bytes: &[u8]) -> Result<(), String> {
        let was_running = self.singbox_host.running_pid().await?.is_some();
        if was_running {
            self.controller.stop_kernel(&mut self.singbox_host).await?;
        }
        self.singbox_host.install_binary(bytes).await?;
        if was_running {
            self.controller
                .start_kernel(
                    &mut self.singbox_host,
                    &self.profile_host,
                    &self.app_config_store,
                )
                .await?;
        }
        Ok(())
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
        self.controller
            .delete_profile(
                id,
                &mut self.singbox_host,
                &self.profile_host,
                &self.app_config_store,
            )
            .await
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
        self.controller
            .activate_profile(
                id,
                &mut self.singbox_host,
                &self.profile_host,
                &self.app_config_store,
            )
            .await
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
        self.controller
            .auto_start_kernel_if_needed(
                &mut self.singbox_host,
                &self.profile_host,
                &self.app_config_store,
            )
            .await
    }

    pub async fn shutdown_kernel_on_exit(&mut self) -> Result<(), String> {
        self.controller
            .shutdown_kernel_on_exit(&mut self.singbox_host)
            .await
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
    let result = {
        let template = {
            let guard = runtime.lock().await;
            guard
                .controller
                .state
                .gui_config
                .templates
                .iter()
                .find(|item| item.id == profile.template_id)
                .map(|item| item.content.clone())
        }
        .ok_or_else(|| String::from("Profile references a missing Template."))?;
        let remotes = profile.remotes.clone();
        let multi_remote = remotes.len() > 1;
        let mut snapshots = Vec::new();
        for remote in &remotes {
            let snapshot = match AppController::download_profile(&remote.url, &remote.headers).await
            {
                Ok(content) => {
                    let snapshot = parse_remote(
                        &RemoteSource {
                            name: remote.name.clone(),
                            url: remote.url.clone(),
                            format: match remote.format {
                                crate::state::ProfileRemoteFormat::Clash => RemoteFormat::Clash,
                                crate::state::ProfileRemoteFormat::Singbox => RemoteFormat::Singbox,
                            },
                            keep: RemoteKeepFields {
                                nodes: remote.keep.nodes,
                                groups: remote.keep.groups,
                                route_final: remote.keep.route_final,
                                route_rules: remote.keep.route_rules,
                            },
                        },
                        &content,
                        multi_remote,
                    )?;
                    for warning in &snapshot.warnings {
                        log::warn!("{warning}");
                    }
                    profile_host
                        .save_remote_raw(&profile.id, &remote.name, &content)
                        .await?;
                    log::info!(
                        "Remote fetch succeeded: profile_id={} remote={} url={}",
                        profile.id,
                        remote.name,
                        remote.url
                    );
                    snapshot
                }
                Err(error) => {
                    log::warn!(
                        "Remote fetch failed: profile_id={} remote={} url={} error={error}; attempting to use the most recent raw cache",
                        profile.id,
                        remote.name,
                        remote.url
                    );
                    let Some(cached) = profile_host
                        .read_remote_raw(&profile.id, &remote.name)
                        .await?
                    else {
                        return Err(format!(
                            "Remote {} refresh failed and no cache is available: {error}",
                            remote.name
                        ));
                    };
                    parse_remote(
                        &RemoteSource {
                            name: remote.name.clone(),
                            url: remote.url.clone(),
                            format: match remote.format {
                                crate::state::ProfileRemoteFormat::Clash => RemoteFormat::Clash,
                                crate::state::ProfileRemoteFormat::Singbox => RemoteFormat::Singbox,
                            },
                            keep: RemoteKeepFields {
                                nodes: remote.keep.nodes,
                                groups: remote.keep.groups,
                                route_final: remote.keep.route_final,
                                route_rules: remote.keep.route_rules,
                            },
                        },
                        &cached,
                        multi_remote,
                    )
                    .map_err(|err| {
                        format!(
                            "Failed to parse raw cache for Remote {}: {err}",
                            remote.name
                        )
                    })?
                }
            };
            snapshots.push(snapshot);
        }
        build_config(&template, snapshots, profile.hook.as_deref()).map_err(
                |error| {
                    log::error!(
                        "Failed to generate Profile runtime configuration: profile_id={} remotes={} error={error}",
                        profile.id,
                        remotes.len()
                    );
                    error
                },
            )
    };

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
        Ok(content) => {
            guard
                .profile_host
                .save_runtime(&profile.id, &content)
                .await?;
            {
                let target = &mut guard.controller.state.gui_config.profiles[index];
                target.updated_at = now;
                target.last_attempt_at = now;
                target.last_update_error = None;
                target.next_update_at = next_update_at;
            }
            guard
                .app_config_store
                .save(&guard.controller.state.gui_config)
                .await?;

            if restart_current_kernel && is_current {
                let GuiRuntime {
                    controller,
                    singbox_host,
                    profile_host,
                    app_config_store,
                    ..
                } = &mut *guard;

                // Refresh is explicitly asked to apply the active Profile to the
                // core. The status may be stale when the core exited between UI
                // snapshots, so synchronize it before deciding whether a stop is
                // needed. `start_kernel` then either restarts a live core or starts
                // the active Profile from its newly persisted runtime config.
                controller.sync_runtime(singbox_host).await;
                if controller.state.kernel.status == KernelStatus::Running {
                    controller.stop_kernel(singbox_host).await?;
                }
                controller
                    .start_kernel(singbox_host, profile_host, app_config_store)
                    .await?;
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
                .save(&guard.controller.state.gui_config)
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
