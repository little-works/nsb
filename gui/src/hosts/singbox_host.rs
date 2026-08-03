use std::{
    fs::{self, File},
    net::{Ipv4Addr, SocketAddr, TcpListener},
    path::{Path, PathBuf},
    process,
    process::{Child, Command as StdCommand, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use log::info;
use nsb_core::{ClashApi, Experimental, Inbound, Log, SingBoxConfig};
use serde::{Deserialize, Serialize};
use tokio::fs as tokio_fs;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::app::profile_builder::run_finalize_hook;
use crate::config::AppConfig;
use crate::utils::command::command;
use crate::utils::path::ensure_data_dir;

static SECRET_COUNTER: AtomicU64 = AtomicU64::new(0);
const CONTROLLER_START_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub external_controller: String,
    pub secret: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct KernelProcessMarker {
    pid: u32,
    created_at: Option<u64>,
}

pub struct SingBoxHost {
    data_dir: PathBuf,
    workspace_dir: PathBuf,
    kernel_dir: PathBuf,
    binary_path: PathBuf,
    config_path: PathBuf,
    pid_path: PathBuf,
    log_path: PathBuf,
    child: Option<Child>,
}

impl SingBoxHost {
    pub fn detect() -> Result<Self, String> {
        let data_dir = ensure_data_dir("")?;
        let workspace_dir = ensure_data_dir("runtime/workspace")?;
        let kernel_dir = ensure_data_dir("sing-box")?;
        let binary_path = kernel_dir.join(binary_name());
        let config_path = workspace_dir.join("config.json");
        let pid_path = workspace_dir.join("pid.txt");
        let log_path = data_dir.join("logs").join("sing-box.log");
        clear_log_file(&log_path)?;

        Ok(Self {
            data_dir,
            workspace_dir,
            kernel_dir,
            binary_path,
            config_path,
            pid_path,
            log_path,
            child: None,
        })
    }

    pub fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn display_data_dir(&self) -> String {
        if self.is_dev_mode() {
            String::from("@gui/dev-data")
        } else {
            self.data_dir.display().to_string()
        }
    }

    pub fn display_config_path(&self) -> String {
        if self.is_dev_mode() {
            String::from("@gui/dev-data/runtime/workspace/config.json")
        } else {
            self.config_path.display().to_string()
        }
    }

    pub async fn read_version(&self) -> Result<String, String> {
        Self::read_version_for_binary(self.binary_path.clone()).await
    }

    pub async fn read_version_for_binary(binary_path: PathBuf) -> Result<String, String> {
        ensure_kernel_binary_exists(&binary_path)?;
        let mut command = command(&binary_path);
        let output = command
            .arg("version")
            .output()
            .await
            .map_err(|err| format!("Failed to read sing-box version: {err}"))?;

        if !output.status.success() {
            return Err(format!(
                "Failed to read sing-box version; exit status: {}",
                output.status
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .next()
            .unwrap_or("sing-box")
            .trim()
            .to_string())
    }

    pub async fn install_binary(&self, bytes: &[u8]) -> Result<(), String> {
        self.ensure_kernel_dir()?;
        let temporary_path = self.kernel_dir.join(format!(
            ".sing-box-import-{}-{}{}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            if cfg!(windows) { ".exe" } else { "" },
        ));
        let backup_path = self.kernel_dir.join(format!("{}.bak", binary_name()));

        tokio_fs::write(&temporary_path, bytes)
            .await
            .map_err(|err| format!("Failed to write imported sing-box binary: {err}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary_path, fs::Permissions::from_mode(0o755))
                .map_err(|err| format!("Failed to set sing-box executable permissions: {err}"))?;
        }

        let _ = tokio_fs::remove_file(&backup_path).await;
        if self.binary_path.exists() {
            tokio_fs::rename(&self.binary_path, &backup_path)
                .await
                .map_err(|err| format!("Failed to back up current sing-box core: {err}"))?;
        }

        if let Err(err) = tokio_fs::rename(&temporary_path, &self.binary_path).await {
            if backup_path.exists() {
                let _ = tokio_fs::rename(&backup_path, &self.binary_path).await;
            }
            return Err(format!("Failed to install imported sing-box core: {err}"));
        }

        let _ = tokio_fs::remove_file(backup_path).await;
        Ok(())
    }

    pub async fn start(
        &mut self,
        source: &str,
        gui_config: &AppConfig,
        hook: Option<&str>,
    ) -> Result<LaunchConfig, String> {
        if self.running_pid().await?.is_some() {
            return Err(String::from("sing-box is already running."));
        }

        self.ensure_kernel_exists()?;
        let launch_config = self.materialize_config(source, gui_config, hook).await?;

        let log_file = prepare_log_file(&self.log_path)?;
        let err_log_file = log_file
            .try_clone()
            .map_err(|err| format!("Failed to clone log file handle: {err}"))?;

        let mut command = StdCommand::new(&self.binary_path);
        configure_command(&mut command);
        let mut child = command
            .arg("run")
            .arg("--disable-color")
            .arg("-D")
            .arg(&self.workspace_dir)
            .arg("-c")
            .arg(&self.config_path)
            .current_dir(&self.kernel_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(err_log_file))
            .spawn()
            .map_err(|err| format!("Failed to start sing-box: {err}"))?;

        self.wait_until_ready(&mut child, &launch_config).await?;

        if let Err(err) = self.write_process_marker(child.id()).await {
            let mut child = child;
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }
        self.child = Some(child);
        info!("sing-box started successfully");
        Ok(launch_config)
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.clear_pid_file().await?;
                    return Ok(());
                }
                Ok(None) => {
                    child
                        .kill()
                        .map_err(|err| format!("Failed to stop sing-box: {err}"))?;
                    child.wait().map_err(|err| {
                        format!("Failed while waiting for sing-box to exit: {err}")
                    })?;
                    self.clear_pid_file().await?;
                    return Ok(());
                }
                Err(err) => {
                    self.child = Some(child);
                    return Err(format!("Failed to check sing-box process status: {err}"));
                }
            }
        }

        self.child = None;
        self.clear_pid_file().await?;
        Ok(())
    }

    pub async fn cleanup_orphaned_kernel(&self) -> Result<(), String> {
        let Some(marker) = self.read_process_marker().await? else {
            return Ok(());
        };

        if process_matches_marker(&marker) {
            kill_process_by_pid(marker.pid)?;
        }
        self.clear_pid_file().await
    }

    pub async fn running_pid(&mut self) -> Result<Option<u32>, String> {
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.clear_pid_file().await?;
                }
                Ok(None) => {
                    let pid = child.id();
                    self.child = Some(child);
                    return Ok(Some(pid));
                }
                Err(err) => {
                    self.child = Some(child);
                    return Err(format!("Failed to check sing-box process status: {err}"));
                }
            }
        }

        Ok(None)
    }

    pub async fn load_launch_config(&self) -> Result<Option<LaunchConfig>, String> {
        if !tokio_fs::try_exists(&self.config_path)
            .await
            .map_err(|err| {
                format!(
                    "Failed to check whether configuration file exists: {}: {err}",
                    self.config_path.display()
                )
            })?
        {
            return Ok(None);
        }

        let body = tokio_fs::read_to_string(&self.config_path)
            .await
            .map_err(|err| {
                format!(
                    "Failed to read configuration file: {}: {err}",
                    self.config_path.display()
                )
            })?;
        let config = serde_json::from_str::<SingBoxConfig>(&body).map_err(|err| {
            format!(
                "Failed to parse configuration file: {}: {err}",
                self.config_path.display()
            )
        })?;
        let clash_api = config
            .experimental
            .as_ref()
            .and_then(|experimental| experimental.clash_api.as_ref());

        clash_api
            .map(|_| launch_config_from_config(&config))
            .transpose()
    }

    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    pub async fn materialize_config(
        &self,
        source: &str,
        gui_config: &AppConfig,
        hook: Option<&str>,
    ) -> Result<LaunchConfig, String> {
        self.ensure_workspace_dir()?;
        let config = self.load_sing_box_config(source).await?;
        let mut config = self.apply_gui_overrides(config, gui_config)?;
        if let Some(hook) = hook.filter(|value| !value.trim().is_empty()) {
            let value = serde_json::to_value(&config).map_err(|err| {
                format!("Failed to serialize final sing-box configuration for Profile hook: {err}")
            })?;
            let value = run_finalize_hook(hook, value)?;
            config = serde_json::from_value(value).map_err(|err| {
                format!("Profile hook onFinalize returned an invalid sing-box configuration: {err}")
            })?;
        }
        let launch_config = launch_config_from_config(&config)?;
        let body = serde_json::to_string_pretty(&config)
            .map_err(|err| format!("Failed to serialize sing-box configuration: {err}"))?;

        tokio_fs::write(&self.config_path, body)
            .await
            .map_err(|err| {
                format!(
                    "Failed to write configuration file: {}: {err}",
                    self.config_path.display()
                )
            })?;

        Ok(launch_config)
    }

    fn ensure_data_dir(&self) -> Result<(), String> {
        ensure_data_dir("").map(|_| ())
    }

    fn ensure_workspace_dir(&self) -> Result<(), String> {
        ensure_data_dir("runtime/workspace").map(|_| ())
    }

    fn ensure_kernel_dir(&self) -> Result<(), String> {
        fs::create_dir_all(&self.kernel_dir).map_err(|err| {
            format!(
                "Failed to create core directory: {}: {err}",
                self.kernel_dir.display()
            )
        })
    }

    fn ensure_kernel_exists(&self) -> Result<(), String> {
        ensure_kernel_binary_exists(&self.binary_path)
    }

    async fn load_sing_box_config(&self, source: &str) -> Result<SingBoxConfig, String> {
        let body = self.read_config_source(source).await?;
        serde_json::from_str(&body)
            .map_err(|err| format!("Failed to parse sing-box configuration: {err}"))
    }

    async fn read_config_source(&self, source: &str) -> Result<String, String> {
        if source.starts_with("http://") || source.starts_with("https://") {
            let response = reqwest::get(source)
                .await
                .map_err(|err| format!("Failed to download subscription configuration: {err}"))?
                .error_for_status()
                .map_err(|err| format!("Subscription returned an error status: {err}"))?;

            response
                .text()
                .await
                .map_err(|err| format!("Failed to read subscription content: {err}"))
        } else {
            tokio_fs::read_to_string(source)
                .await
                .map_err(|err| format!("Failed to read local configuration: {source}: {err}"))
        }
    }

    fn apply_gui_overrides(
        &self,
        mut config: SingBoxConfig,
        gui_config: &AppConfig,
    ) -> Result<SingBoxConfig, String> {
        let controller_port = find_available_local_port()?;
        let controller_addr = format!("127.0.0.1:{controller_port}");
        let secret = generate_controller_secret(controller_port);

        config.inbounds = vec![Inbound {
            type_: String::from("mixed"),
            tag: String::from("mixed-in"),
            listen: if gui_config.allow_lan {
                String::from("0.0.0.0")
            } else {
                String::from("127.0.0.1")
            },
            listen_port: usize::from(gui_config.mixed_port),
            tcp_fast_open: Some(false),
            tcp_multi_path: Some(false),
            udp_fragment: Some(false),
            domain_strategy: None,
            users: None,
            extra: Default::default(),
        }];

        let mut log = config.log.take().unwrap_or(Log {
            disabled: None,
            level: None,
            output: None,
            timestamp: None,
        });
        log.timestamp = Some(true);
        config.log = Some(log);

        let mut experimental = config.experimental.take().unwrap_or(Experimental {
            clash_api: None,
            cache_file: None,
        });
        let mut clash_api = experimental.clash_api.take().unwrap_or(ClashApi {
            external_controller: None,
            external_ui: None,
            external_ui_download_url: None,
            external_ui_download_detour: None,
            secret: None,
            default_mode: None,
            access_control_allow_origin: None,
            access_control_allow_private_network: None,
        });
        clash_api.external_controller = Some(controller_addr.clone());
        clash_api.secret = Some(secret.clone());
        experimental.clash_api = Some(clash_api);
        config.experimental = Some(experimental);

        Ok(config)
    }

    fn is_dev_mode(&self) -> bool {
        self.data_dir == PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dev-data")
    }

    async fn read_process_marker(&self) -> Result<Option<KernelProcessMarker>, String> {
        if !tokio_fs::try_exists(&self.pid_path).await.map_err(|err| {
            format!(
                "Failed to check whether PID file exists: {}: {err}",
                self.pid_path.display()
            )
        })? {
            return Ok(None);
        }

        let raw = tokio_fs::read_to_string(&self.pid_path)
            .await
            .map_err(|err| {
                format!(
                    "Failed to read PID file: {}: {err}",
                    self.pid_path.display()
                )
            })?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        match serde_json::from_str::<KernelProcessMarker>(trimmed) {
            Ok(marker) => Ok(Some(marker)),
            Err(_) => {
                self.clear_pid_file().await?;
                Ok(None)
            }
        }
    }

    async fn write_process_marker(&self, pid: u32) -> Result<(), String> {
        self.ensure_workspace_dir()?;
        let marker = KernelProcessMarker {
            pid,
            created_at: process_creation_time(pid).ok(),
        };
        let body = serde_json::to_string(&marker)
            .map_err(|err| format!("Failed to serialize core process marker: {err}"))?;
        tokio_fs::write(&self.pid_path, body).await.map_err(|err| {
            format!(
                "Failed to write core process marker: {}: {err}",
                self.pid_path.display()
            )
        })
    }

    async fn clear_pid_file(&self) -> Result<(), String> {
        if !tokio_fs::try_exists(&self.pid_path).await.map_err(|err| {
            format!(
                "Failed to check whether PID file exists: {}: {err}",
                self.pid_path.display()
            )
        })? {
            return Ok(());
        }

        tokio_fs::remove_file(&self.pid_path).await.map_err(|err| {
            format!(
                "Failed to delete PID file: {}: {err}",
                self.pid_path.display()
            )
        })
    }

    async fn wait_until_ready(
        &self,
        child: &mut Child,
        launch_config: &LaunchConfig,
    ) -> Result<(), String> {
        let deadline =
            tokio::time::Instant::now() + Duration::from_secs(CONTROLLER_START_TIMEOUT_SECS);
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|err| format!("Failed to check sing-box startup status: {err}"))?
            {
                let detail = last_log_line(&self.log_path);
                return Err(match detail {
                    Some(detail) => {
                        format!(
                            "sing-box exited immediately after startup (exit status: {status}): {detail}"
                        )
                    }
                    None => format!(
                        "sing-box exited immediately after startup (exit status: {status})."
                    ),
                });
            }

            if tokio::net::TcpStream::connect(&launch_config.external_controller)
                .await
                .is_ok()
            {
                return Ok(());
            }

            if tokio::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Timed out waiting for sing-box controller to start ({} seconds).",
                    CONTROLLER_START_TIMEOUT_SECS
                ));
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

fn launch_config_from_config(config: &SingBoxConfig) -> Result<LaunchConfig, String> {
    let clash_api = config
        .experimental
        .as_ref()
        .and_then(|experimental| experimental.clash_api.as_ref())
        .ok_or_else(|| {
            String::from("Final sing-box configuration is missing experimental.clash_api.")
        })?;
    let external_controller = clash_api
        .external_controller
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            String::from(
                "Final sing-box configuration requires a non-empty experimental.clash_api.external_controller.",
            )
        })?;
    let controller = external_controller.parse::<SocketAddr>().map_err(|_| {
        format!(
            "Final sing-box configuration has an invalid Clash API external_controller: {external_controller}. Use 127.0.0.1:<port> or 0.0.0.0:<port>."
        )
    })?;
    if controller.port() == 0
        || !matches!(
            controller.ip(),
            std::net::IpAddr::V4(ip) if ip == Ipv4Addr::LOCALHOST || ip == Ipv4Addr::UNSPECIFIED
        )
    {
        return Err(format!(
            "Final sing-box configuration has an unsupported Clash API external_controller: {external_controller}. Use 127.0.0.1:<port> or 0.0.0.0:<port>."
        ));
    }
    let secret = clash_api
        .secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            String::from(
                "Final sing-box configuration requires a non-empty experimental.clash_api.secret.",
            )
        })?;

    Ok(LaunchConfig {
        external_controller: format!("127.0.0.1:{}", controller.port()),
        secret: secret.to_string(),
    })
}

fn ensure_kernel_binary_exists(binary_path: &Path) -> Result<(), String> {
    if binary_path.exists() {
        Ok(())
    } else {
        Err(format!(
            "sing-box executable was not found: {}",
            binary_path.display()
        ))
    }
}

fn prepare_log_file(path: &Path) -> Result<File, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create log directory: {}: {err}",
                parent.display()
            )
        })?;
    }

    File::options()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("Failed to open log file: {}: {err}", path.display()))
}

fn clear_log_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Failed to create log directory: {}: {err}",
                parent.display()
            )
        })?;
    }

    File::create(path)
        .map(|_| ())
        .map_err(|err| format!("Failed to clear log file: {}: {err}", path.display()))
}

fn last_log_line(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()?
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "sing-box.exe"
    } else {
        "sing-box"
    }
}

fn configure_command(command: &mut StdCommand) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn process_matches_singbox(pid: u32) -> bool {
    if let Ok(name) = process_name_by_pid(pid) {
        return matches_singbox_name(&name);
    }

    false
}

fn process_matches_marker(marker: &KernelProcessMarker) -> bool {
    if !process_matches_singbox(marker.pid) {
        return false;
    }

    match marker.created_at {
        Some(expected) => process_creation_time(marker.pid).is_ok_and(|actual| actual == expected),
        None => !cfg!(windows),
    }
}

#[cfg(windows)]
fn process_creation_time(pid: u32) -> Result<u64, String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(format!(
            "Failed to open process {pid}: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut created = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exited = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let success =
        unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) };
    unsafe {
        CloseHandle(handle);
    }
    if success == 0 {
        return Err(format!(
            "Failed to read creation time for process {pid}: {}",
            std::io::Error::last_os_error()
        ));
    }

    Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

#[cfg(not(windows))]
fn process_creation_time(_pid: u32) -> Result<u64, String> {
    Err(String::from(
        "Reading process creation time is not supported on this platform.",
    ))
}

fn process_name_by_pid(pid: u32) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let filter = format!("PID eq {pid}");
        let mut command = StdCommand::new("tasklist");
        configure_command(&mut command);
        let output = command
            .args(["/FI", &filter, "/FO", "CSV", "/NH"])
            .output()
            .map_err(|err| format!("Failed to read process information: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "Failed to read process information; exit status: {}",
                output.status
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().map(str::trim).find(|line| !line.is_empty());
        let Some(line) = line else {
            return Err(String::from(
                "Corresponding process information was not found",
            ));
        };
        if line
            .eq_ignore_ascii_case("INFO: No tasks are running which match the specified criteria.")
        {
            return Err(String::from("Process does not exist"));
        }

        let name = line
            .trim_matches('"')
            .split("\",\"")
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        if name.is_empty() {
            return Err(String::from("Process name is empty"));
        }
        return Ok(name);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = StdCommand::new("ps")
            .args(["-o", "comm=", "-p", &pid.to_string()])
            .output()
            .map_err(|err| format!("Failed to read process information: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "Failed to read process information; exit status: {}",
                output.status
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let name = stdout.trim();
        if name.is_empty() {
            return Err(String::from("Process does not exist"));
        }
        Ok(name.to_string())
    }
}

fn matches_singbox_name(name: &str) -> bool {
    Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            value.eq_ignore_ascii_case(binary_name()) || value.starts_with("sing-box")
        })
}

fn kill_process_by_pid(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut command = StdCommand::new("taskkill");
        configure_command(&mut command);
        let output = command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map_err(|err| format!("Failed to stop sing-box: {err}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            if detail.is_empty() {
                return Err(format!(
                    "Failed to stop sing-box; exit status: {}",
                    output.status
                ));
            }
            return Err(format!("Failed to stop sing-box: {detail}"));
        }
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = StdCommand::new("kill")
            .arg(pid.to_string())
            .output()
            .map_err(|err| format!("Failed to stop sing-box: {err}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            if detail.is_empty() {
                return Err(format!(
                    "Failed to stop sing-box; exit status: {}",
                    output.status
                ));
            }
            return Err(format!("Failed to stop sing-box: {detail}"));
        }
        Ok(())
    }
}

fn find_available_local_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|err| format!("Failed to allocate clash_api port: {err}"))?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|err| format!("Failed to read clash_api port: {err}"))
}

fn generate_controller_secret(port: u16) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = SECRET_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = u64::from(process::id());
    format!("{nanos:032x}{pid:08x}{counter:08x}{port:04x}")
}

#[cfg(test)]
mod tests {
    use nsb_core::{ClashApi, Experimental, SingBoxConfig};

    use super::launch_config_from_config;

    fn config_with_controller(controller: Option<&str>, secret: Option<&str>) -> SingBoxConfig {
        let mut config = SingBoxConfig::default();
        config.experimental = Some(Experimental {
            clash_api: Some(ClashApi {
                external_controller: controller.map(String::from),
                secret: secret.map(String::from),
                ..Default::default()
            }),
            cache_file: None,
        });
        config
    }

    #[test]
    fn accepts_a_loopback_controller_address() {
        let config = config_with_controller(Some("127.0.0.1:9090"), Some("secret"));

        let launch_config = launch_config_from_config(&config).unwrap();

        assert_eq!(launch_config.external_controller, "127.0.0.1:9090");
        assert_eq!(launch_config.secret, "secret");
    }

    #[test]
    fn normalizes_an_unspecified_controller_address_to_loopback() {
        let config = config_with_controller(Some("0.0.0.0:9090"), Some("secret"));

        let launch_config = launch_config_from_config(&config).unwrap();

        assert_eq!(launch_config.external_controller, "127.0.0.1:9090");
    }

    #[test]
    fn rejects_an_unsupported_controller_host() {
        let config = config_with_controller(Some("192.168.1.2:9090"), Some("secret"));

        let error = launch_config_from_config(&config).unwrap_err();

        assert!(error.contains("unsupported Clash API external_controller"));
    }

    #[test]
    fn rejects_a_missing_controller_secret() {
        let config = config_with_controller(Some("127.0.0.1:9090"), None);

        let error = launch_config_from_config(&config).unwrap_err();

        assert!(error.contains("requires a non-empty experimental.clash_api.secret"));
    }
}
