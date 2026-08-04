use log::{info, warn};
use std::str::FromStr;

use crate::config::{AppConfig, AppConfigStore, AppLanguage};
use crate::hosts::system_proxy_host::SystemProxyHost;
use crate::hosts::{ProfileHost, SingBoxHost};
use crate::state::{
    AppState, KernelRuntimeSource, ProfileHeader, ProfileItem, ProfileKind, ProfileRemote,
    current_timestamp, generate_profile_id,
};

const PROFILE_USER_AGENT: &str = concat!("ClashforWindows/0.20.16 NSB/", env!("CARGO_PKG_VERSION"));

pub struct AppController {
    pub state: AppState,
    kernel_started_by_this_instance: bool,
}

struct PreparedProfile {
    kind: ProfileKind,
    url: String,
    runtime_content: Option<String>,
}

impl AppController {
    pub fn new(gui_config: AppConfig) -> Self {
        Self {
            state: AppState::load(gui_config),
            kernel_started_by_this_instance: false,
        }
    }

    pub async fn bootstrap_runtime(
        &mut self,
        singbox_host: &mut SingBoxHost,
        profile_host: &ProfileHost,
    ) {
        self.state.kernel.binary_path = singbox_host.binary_path().display().to_string();
        self.state.kernel.installed = singbox_host.is_installed();
        self.state.kernel.data_dir = singbox_host.display_data_dir();
        self.state.kernel.config_path = singbox_host.display_config_path();

        for profile in &mut self.state.gui_config.profiles {
            profile.kind = if profile.url.trim().is_empty() {
                ProfileKind::File
            } else {
                ProfileItem::source_kind(&profile.url)
            };
            if profile.id.trim().is_empty() {
                profile.id = generate_profile_id();
            }
            let _ = profile_host.migrate_runtime(&profile.id).await;
        }
        self.normalize_current_profile();

        self.state.kernel.version = match singbox_host.read_version().await {
            Ok(version) => version,
            Err(_) => String::from("Detection failed"),
        };

        if let Err(err) = singbox_host.cleanup_orphaned_kernel().await {
            warn!("failed to clean up sing-box left by a previous crash: {err}");
        }
        self.sync_kernel_runtime(singbox_host).await;
    }

    pub async fn sync_runtime(&mut self, singbox_host: &mut SingBoxHost) {
        self.sync_kernel_runtime(singbox_host).await;
    }

    pub async fn start_kernel(
        &mut self,
        singbox_host: &mut SingBoxHost,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        self.sync_kernel_runtime(singbox_host).await;
        if self.state.kernel.status == crate::state::KernelStatus::Running {
            return Err(String::from("The sing-box core is already running."));
        }

        let current = self
            .current_profile()
            .cloned()
            .ok_or_else(|| String::from("No Profile is active. Add and select a Profile first."))?;
        let path = self
            .ensure_profile_runtime(&current, profile_host, app_config_store)
            .await?;
        let source = path.display().to_string();
        match singbox_host
            .start(&source, &self.state.gui_config, current.hook.as_deref())
            .await
        {
            Ok(launch_config) => {
                self.state.kernel.controller_addr = launch_config.external_controller;
                self.state.kernel.controller_secret = launch_config.secret;
                info!(
                    "kernel started, controller={}, mixed_port={}, allow_lan={}",
                    self.state.kernel.controller_addr,
                    self.state.gui_config.mixed_port,
                    self.state.gui_config.allow_lan
                );
                self.state.mark_kernel_running();
                self.kernel_started_by_this_instance = true;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub async fn stop_kernel(&mut self, singbox_host: &mut SingBoxHost) -> Result<(), String> {
        self.sync_kernel_runtime(singbox_host).await;
        if self.state.kernel.status != crate::state::KernelStatus::Running {
            self.state.mark_kernel_stopped();
            return Ok(());
        }

        singbox_host.stop().await?;
        self.state.mark_kernel_stopped();
        self.kernel_started_by_this_instance = false;
        self.state.kernel.controller_addr = String::from("Assigned randomly at startup");
        self.state.kernel.controller_secret = String::from("Generated at startup");
        Ok(())
    }

    pub async fn toggle_kernel(
        &mut self,
        singbox_host: &mut SingBoxHost,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<bool, String> {
        self.sync_kernel_runtime(singbox_host).await;
        if self.state.kernel.status == crate::state::KernelStatus::Running {
            self.stop_kernel(singbox_host).await?;
            Ok(false)
        } else {
            self.start_kernel(singbox_host, profile_host, app_config_store)
                .await?;
            Ok(true)
        }
    }

    pub async fn create_profile(
        &mut self,
        name: String,
        source: String,
        content: Option<String>,
        headers: Vec<ProfileHeader>,
        update_interval_hours: Option<u32>,
        update_cron: Option<String>,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        let name = Self::validate_profile_name(name)?;
        let id = self.next_profile_id();
        Self::validate_headers(&headers)?;
        Self::validate_schedule(update_interval_hours, update_cron.as_deref())?;
        let prepared = Self::prepare_profile(None, source, content)?;
        if let Some(runtime_content) = prepared.runtime_content.as_deref() {
            profile_host.save_runtime(&id, runtime_content).await?;
        }
        let next_update_at = Self::next_update_at(
            &prepared.kind,
            update_interval_hours,
            update_cron.as_deref(),
        );

        let item = ProfileItem {
            id: id.clone(),
            name,
            kind: prepared.kind,
            url: prepared.url,
            updated_at: current_timestamp(),
            headers,
            remotes: Vec::new(),
            hook: None,
            keep_subscription_groups_and_rules: false,
            update_interval_hours,
            update_cron,
            next_update_at,
            last_attempt_at: 0,
            last_update_error: None,
            revision: 1,
        };

        self.state.gui_config.profiles.push(item);
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn update_profile(
        &mut self,
        id: String,
        name: String,
        source: String,
        content: Option<String>,
        headers: Vec<ProfileHeader>,
        update_interval_hours: Option<u32>,
        update_cron: Option<String>,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err(String::from("Profile ID cannot be empty."));
        }
        let name = Self::validate_profile_name(name)?;
        Self::validate_headers(&headers)?;

        let existing_index = self
            .state
            .gui_config
            .profiles
            .iter()
            .position(|profile| profile.id == id)
            .ok_or_else(|| String::from("Profile to update was not found."))?;

        let existing = self.state.gui_config.profiles[existing_index].clone();
        Self::validate_schedule(update_interval_hours, update_cron.as_deref())?;
        let prepared = Self::prepare_profile(Some(&existing), source, content)?;
        if let Some(runtime_content) = prepared.runtime_content.as_deref() {
            profile_host
                .save_runtime(&existing.id, runtime_content)
                .await?;
        }

        let target = &mut self.state.gui_config.profiles[existing_index];
        target.name = name;
        target.kind = prepared.kind;
        target.url = prepared.url;
        target.headers = headers;
        target.update_interval_hours = update_interval_hours;
        target.update_cron = update_cron;
        target.next_update_at = Self::next_update_at(
            &target.kind,
            target.update_interval_hours,
            target.update_cron.as_deref(),
        );
        target.updated_at = current_timestamp();
        target.revision = target.revision.saturating_add(1);

        self.normalize_current_profile();
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn delete_profile(
        &mut self,
        id: String,
        singbox_host: &mut SingBoxHost,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err(String::from("Profile ID cannot be empty."));
        }

        let Some(index) = self
            .state
            .gui_config
            .profiles
            .iter()
            .position(|profile| profile.id == id)
        else {
            return Err(String::from("Profile to delete was not found."));
        };

        let removed = self.state.gui_config.profiles[index].clone();
        let is_current =
            self.state.gui_config.current_profile_id.as_deref() == Some(removed.id.as_str());
        self.sync_kernel_runtime(singbox_host).await;
        if is_current && self.state.kernel.status == crate::state::KernelStatus::Running {
            self.stop_kernel(singbox_host).await?;
        }

        profile_host.delete_runtime(&removed.id).await?;

        self.state.gui_config.profiles.remove(index);
        if is_current {
            self.state.gui_config.current_profile_id = None;
        }

        self.normalize_current_profile();
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn configure_profile_sources(
        &mut self,
        id: &str,
        remotes: Vec<ProfileRemote>,
        hook: Option<String>,
        keep_subscription_groups_and_rules: bool,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        for remote in &remotes {
            Self::validate_headers(&remote.headers)?;
        }
        if remotes.len() > 1 {
            let mut names = std::collections::HashSet::new();
            for remote in &remotes {
                let name = remote.name.trim();
                if name.is_empty()
                    || !names.insert(name.to_string())
                    || matches!(name, "PROXY" | "direct" | "block")
                {
                    return Err(String::from(
                        "Remote names must be unique and cannot be PROXY, direct, or block.",
                    ));
                }
            }
        }
        let profile = self.find_profile(id)?;
        let index = self
            .state
            .gui_config
            .profiles
            .iter()
            .position(|item| item.id == profile.id)
            .expect("profile exists");
        let target = &mut self.state.gui_config.profiles[index];
        if !remotes.is_empty() {
            target.url = remotes[0].url.clone();
            target.headers = remotes[0].headers.clone();
            target.kind = ProfileKind::Url;
            target.remotes = remotes;
        }
        target.hook = hook
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        target.keep_subscription_groups_and_rules = keep_subscription_groups_and_rules;
        target.revision = target.revision.saturating_add(1);
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn read_profile_runtime(
        &self,
        id: &str,
        profile_host: &ProfileHost,
    ) -> Result<String, String> {
        let profile = self.find_profile(id)?;
        profile_host.read_runtime(&profile.id).await
    }

    pub async fn save_profile_runtime(
        &mut self,
        id: &str,
        content: String,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        let profile = self.find_profile(id)?.clone();
        serde_json::from_str::<nsb_core::SingBoxConfig>(&content)
            .map_err(|err| format!("Invalid Profile configuration format: {err}"))?;
        profile_host.save_runtime(&profile.id, &content).await?;

        if let Some(stored_profile) = self
            .state
            .gui_config
            .profiles
            .iter_mut()
            .find(|stored_profile| stored_profile.id == profile.id)
        {
            stored_profile.updated_at = current_timestamp();
        }
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn set_current_profile(
        &mut self,
        id: String,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err(String::from("Profile ID cannot be empty."));
        }

        if !self
            .state
            .gui_config
            .profiles
            .iter()
            .any(|profile| profile.id == id)
        {
            return Err(String::from("Profile to activate was not found."));
        }

        self.state.gui_config.current_profile_id = Some(id);
        self.normalize_current_profile();
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn activate_profile(
        &mut self,
        id: String,
        singbox_host: &mut SingBoxHost,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        self.sync_kernel_runtime(singbox_host).await;
        let was_kernel_running = self.state.kernel.status == crate::state::KernelStatus::Running;
        let id = id.trim().to_string();
        let profile = self
            .state
            .gui_config
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
            .ok_or_else(|| String::from("Profile to activate was not found."))?;

        self.ensure_profile_runtime(&profile, profile_host, app_config_store)
            .await?;
        self.set_current_profile(id, app_config_store).await?;

        if was_kernel_running {
            self.stop_kernel(singbox_host).await?;
        }
        self.start_kernel(singbox_host, profile_host, app_config_store)
            .await
    }

    pub async fn save_runtime_settings(
        &mut self,
        mixed_port: u16,
        app_port: u16,
        allow_lan: bool,
        system_proxy_enabled: bool,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        if mixed_port == 0 {
            return Err(String::from("The mixed inbound port cannot be 0."));
        }
        if app_port == 0 {
            return Err(String::from("The application port cannot be 0."));
        }

        SystemProxyHost::configure(system_proxy_enabled, mixed_port)?;
        self.state.gui_config.mixed_port = mixed_port;
        self.state.gui_config.app_port = app_port;
        self.state.gui_config.allow_lan = allow_lan;
        self.state.gui_config.system_proxy_enabled = system_proxy_enabled;
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn save_app_language(
        &mut self,
        app_language: AppLanguage,
        app_config_store: &AppConfigStore,
    ) -> Result<(), String> {
        self.state.gui_config.app_language = app_language;
        app_config_store.save(&self.state.gui_config).await
    }

    pub async fn auto_start_kernel_if_needed(
        &mut self,
        singbox_host: &mut SingBoxHost,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<bool, String> {
        self.sync_kernel_runtime(singbox_host).await;
        if self.state.kernel.status == crate::state::KernelStatus::Running {
            return Ok(false);
        }

        self.start_kernel(singbox_host, profile_host, app_config_store)
            .await?;
        Ok(true)
    }

    pub async fn shutdown_kernel_on_exit(
        &mut self,
        singbox_host: &mut SingBoxHost,
    ) -> Result<(), String> {
        self.sync_kernel_runtime(singbox_host).await;
        if !self.kernel_started_by_this_instance {
            return Ok(());
        }
        self.stop_kernel(singbox_host).await
    }

    fn current_profile(&self) -> Option<&ProfileItem> {
        let current_id = self.state.gui_config.current_profile_id.as_deref()?;
        self.state
            .gui_config
            .profiles
            .iter()
            .find(|profile| profile.id == current_id)
    }

    fn find_profile(&self, id: &str) -> Result<&ProfileItem, String> {
        let id = id.trim();
        if id.is_empty() {
            return Err(String::from("Profile ID cannot be empty."));
        }

        self.state
            .gui_config
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or_else(|| String::from("Specified Profile was not found."))
    }

    async fn ensure_profile_runtime(
        &mut self,
        profile: &ProfileItem,
        profile_host: &ProfileHost,
        app_config_store: &AppConfigStore,
    ) -> Result<std::path::PathBuf, String> {
        let path = profile_host.runtime_path(&profile.id);

        if profile_host.runtime_exists(&profile.id).await? {
            return Ok(path);
        }

        match &profile.kind {
            ProfileKind::Url => {
                let body = Self::download_profile(&profile.url, &profile.headers).await?;
                profile_host.save_runtime(&profile.id, &body).await?;
                if let Some(stored_profile) = self
                    .state
                    .gui_config
                    .profiles
                    .iter_mut()
                    .find(|stored_profile| stored_profile.id == profile.id)
                {
                    stored_profile.updated_at = current_timestamp();
                }
                app_config_store.save(&self.state.gui_config).await?;
                Ok(path)
            }
            ProfileKind::File => Err(String::from(
                "Local Profile cache does not exist. Upload the file contents again.",
            )),
        }
    }

    fn normalize_current_profile(&mut self) {
        if self.current_profile().is_some() {
            return;
        }

        self.state.gui_config.current_profile_id = self
            .state
            .gui_config
            .profiles
            .first()
            .map(|profile| profile.id.clone());
    }

    fn next_profile_id(&self) -> String {
        loop {
            let id = generate_profile_id();
            if !self
                .state
                .gui_config
                .profiles
                .iter()
                .any(|profile| profile.id == id)
            {
                return id;
            }
        }
    }

    fn validate_profile_name(name: String) -> Result<String, String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(String::from("Profile name cannot be empty."));
        }
        Ok(name)
    }

    fn prepare_profile(
        existing: Option<&ProfileItem>,
        source: String,
        content: Option<String>,
    ) -> Result<PreparedProfile, String> {
        let source = source.trim().to_string();
        let content = content
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if !source.is_empty() {
            return Ok(PreparedProfile {
                kind: ProfileKind::Url,
                url: source,
                runtime_content: None,
            });
        }

        if let Some(content) = content {
            return Ok(PreparedProfile {
                kind: ProfileKind::File,
                url: String::new(),
                runtime_content: Some(content),
            });
        }

        if existing.is_some_and(|profile| matches!(profile.kind, ProfileKind::File)) {
            return Ok(PreparedProfile {
                kind: ProfileKind::File,
                url: String::new(),
                runtime_content: None,
            });
        }

        Err(String::from(
            "URL cannot be empty; upload the file contents directly for a local file.",
        ))
    }

    fn validate_schedule(
        update_interval_hours: Option<u32>,
        update_cron: Option<&str>,
    ) -> Result<(), String> {
        if update_interval_hours.is_some_and(|hours| hours == 0) {
            return Err(String::from("Update interval must be greater than 0."));
        }
        if update_interval_hours.is_some()
            && update_cron.is_some_and(|cron| !cron.trim().is_empty())
        {
            return Err(String::from(
                "Only one of update interval and Cron can be set.",
            ));
        }
        if let Some(cron) = update_cron.map(str::trim).filter(|cron| !cron.is_empty()) {
            let expression = format!("0 {cron} *");
            cron::Schedule::from_str(&expression)
                .map_err(|err| format!("Invalid Cron expression: {err}"))?;
        }
        Ok(())
    }

    fn next_update_at(
        kind: &ProfileKind,
        update_interval_hours: Option<u32>,
        update_cron: Option<&str>,
    ) -> u64 {
        if !matches!(kind, ProfileKind::Url) {
            return 0;
        }

        let now = current_timestamp();
        if let Some(hours) = update_interval_hours {
            return now.saturating_add(u64::from(hours).saturating_mul(60 * 60));
        }

        let Some(cron) = update_cron.map(str::trim).filter(|cron| !cron.is_empty()) else {
            return 0;
        };
        let expression = format!("0 {cron} *");
        cron::Schedule::from_str(&expression)
            .ok()
            .and_then(|schedule| schedule.after(&chrono::Local::now()).next())
            .and_then(|time| u64::try_from(time.timestamp()).ok())
            .unwrap_or(0)
    }

    fn validate_headers(headers: &[ProfileHeader]) -> Result<(), String> {
        for header in headers {
            if header.key.trim().is_empty() {
                return Err(String::from("Profile Header name cannot be empty."));
            }
            reqwest::header::HeaderName::from_bytes(header.key.trim().as_bytes())
                .map_err(|err| format!("Invalid Profile Header name: {err}"))?;
            reqwest::header::HeaderValue::from_str(&header.value)
                .map_err(|err| format!("Invalid Profile Header value: {err}"))?;
        }
        Ok(())
    }

    pub async fn download_profile(url: &str, headers: &[ProfileHeader]) -> Result<String, String> {
        let mut request = reqwest::Client::new()
            .get(url)
            .header(reqwest::header::USER_AGENT, PROFILE_USER_AGENT);
        for header in headers {
            let name = reqwest::header::HeaderName::from_bytes(header.key.trim().as_bytes())
                .map_err(|err| format!("Invalid Profile Header name: {err}"))?;
            let value = reqwest::header::HeaderValue::from_str(&header.value)
                .map_err(|err| format!("Invalid Profile Header value: {err}"))?;
            request = request.header(name, value);
        }

        let response = request
            .send()
            .await
            .map_err(|err| format!("Failed to download Profile configuration: {err}"))?
            .error_for_status()
            .map_err(|err| format!("Profile returned an error status: {err}"))?;

        response
            .text()
            .await
            .map_err(|err| format!("Failed to read Profile content: {err}"))
    }

    async fn sync_kernel_runtime(&mut self, singbox_host: &mut SingBoxHost) {
        self.state.kernel.installed = singbox_host.is_installed();
        match singbox_host.running_pid().await {
            Ok(Some(pid)) => {
                if let Ok(Some(launch_config)) = singbox_host.load_launch_config().await {
                    self.state.kernel.controller_addr = launch_config.external_controller;
                    self.state.kernel.controller_secret = launch_config.secret;
                }
                self.state.kernel.status = crate::state::KernelStatus::Running;
                self.state.kernel.runtime_source = if self.kernel_started_by_this_instance
                    && singbox_host
                        .child_pid()
                        .is_some_and(|child_pid| child_pid == pid)
                {
                    KernelRuntimeSource::CurrentInstance
                } else {
                    KernelRuntimeSource::PidFile
                };
                if self.state.kernel.last_started_at == "Not started" {
                    self.state.kernel.last_started_at =
                        String::from("Already running in the background");
                }
            }
            Ok(None) => {
                self.state.kernel.status = crate::state::KernelStatus::Stopped;
                self.kernel_started_by_this_instance = false;
                self.state.kernel.controller_addr = String::from("Assigned randomly at startup");
                self.state.kernel.controller_secret = String::from("Generated at startup");
                self.state.kernel.runtime_source = KernelRuntimeSource::None;
            }
            Err(_) => {
                self.state.kernel.status = crate::state::KernelStatus::Stopped;
                self.kernel_started_by_this_instance = false;
                self.state.kernel.controller_addr = String::from("Assigned randomly at startup");
                self.state.kernel.controller_secret = String::from("Generated at startup");
                self.state.kernel.runtime_source = KernelRuntimeSource::None;
            }
        }
    }
}
