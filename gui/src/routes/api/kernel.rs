use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{Multipart, State};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::hosts::SingBoxHost;
use crate::routes::{KernelDownloadProgress, RouteState};
use crate::state::KernelInfo;
use crate::utils::command::std_command;

use super::{ApiResponse, simple_response};

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/SagerNet/sing-box/releases/latest";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Clone, Serialize)]
pub struct KernelReleaseInfo {
    version: String,
}

#[derive(Clone, Serialize)]
pub struct RuntimeStatusResponse {
    pub kernel: KernelInfo,
}

#[derive(Clone, Default, Serialize)]
pub struct KernelDownloadProgressResponse {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub in_progress: bool,
}

pub async fn get_kernel_download_progress(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<KernelDownloadProgressResponse>> {
    let progress = ctx
        .kernel_download_progress
        .lock()
        .map(|progress| KernelDownloadProgressResponse {
            downloaded: progress.downloaded,
            total: progress.total,
            in_progress: progress.in_progress,
        })
        .unwrap_or_default();
    Json(ApiResponse::success(String::new(), Some(progress)))
}

pub async fn get_runtime(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<RuntimeStatusResponse>> {
    let mut guard = ctx.runtime.lock().await;
    guard.sync_runtime().await;
    let kernel = guard.controller.state.kernel.clone();
    Json(ApiResponse::success(
        String::new(),
        Some(RuntimeStatusResponse { kernel }),
    ))
}

struct CachedKernelRelease {
    release: KernelReleaseInfo,
    fetched_at: Instant,
}

static LATEST_KERNEL_RELEASE_CACHE: OnceLock<Mutex<Option<CachedKernelRelease>>> = OnceLock::new();

struct KernelDownloadGuard {
    in_progress: Arc<AtomicBool>,
    progress: Arc<Mutex<KernelDownloadProgress>>,
}

impl Drop for KernelDownloadGuard {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
        if let Ok(mut progress) = self.progress.lock() {
            progress.in_progress = false;
        }
    }
}

pub async fn get_kernel_version(State(ctx): State<RouteState>) -> Json<ApiResponse<String>> {
    let binary_path = {
        let guard = ctx.runtime.lock().await;
        guard.singbox_host.binary_path().to_path_buf()
    };

    simple_response(SingBoxHost::read_version_for_binary(binary_path).await)
}

pub async fn get_latest_kernel_release() -> Json<ApiResponse<KernelReleaseInfo>> {
    if let Some(release) = cached_latest_kernel_release() {
        return simple_response(Ok(release));
    }

    simple_response(fetch_latest_release().await.map(|release| {
        let release = KernelReleaseInfo {
            version: release.tag_name,
        };
        cache_latest_kernel_release(release.clone());
        release
    }))
}

pub async fn toggle_kernel(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<RuntimeStatusResponse>> {
    let mut guard = ctx.runtime.lock().await;
    match guard.toggle_kernel().await {
        Ok(running) => {
            guard.sync_runtime().await;
            let kernel = guard.controller.state.kernel.clone();
            let snapshot = RuntimeStatusResponse { kernel };
            let message = if running {
                String::from("Sing-box core started.")
            } else {
                String::from("Sing-box core stopped.")
            };
            Json(ApiResponse::success(message, Some(snapshot)))
        }
        Err(err) => {
            guard.sync_runtime().await;
            let kernel = guard.controller.state.kernel.clone();
            let snapshot = RuntimeStatusResponse { kernel };
            Json(ApiResponse::failure(err, Some(snapshot)))
        }
    }
}

pub async fn restart_kernel(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<RuntimeStatusResponse>> {
    let mut guard = ctx.runtime.lock().await;
    let result = guard.restart_kernel().await;
    guard.sync_runtime().await;
    let snapshot = RuntimeStatusResponse {
        kernel: guard.controller.state.kernel.clone(),
    };

    match result {
        Ok(()) => Json(ApiResponse::success(
            String::from("Sing-box core restarted."),
            Some(snapshot),
        )),
        Err(err) => Json(ApiResponse::failure(err, Some(snapshot))),
    }
}

pub async fn import_kernel_binary(
    State(ctx): State<RouteState>,
    mut multipart: Multipart,
) -> Json<ApiResponse<String>> {
    let file = loop {
        match multipart.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => match field.bytes().await {
                Ok(bytes) if !bytes.is_empty() => break bytes,
                Ok(_) => {
                    return Json(ApiResponse::failure(
                        String::from("Imported file is empty."),
                        None,
                    ));
                }
                Err(err) => {
                    return Json(ApiResponse::failure(
                        format!("Failed to read imported file: {err}"),
                        None,
                    ));
                }
            },
            Ok(Some(_)) => continue,
            Ok(None) => {
                return Json(ApiResponse::failure(
                    String::from("Select a sing-box executable file."),
                    None,
                ));
            }
            Err(err) => {
                return Json(ApiResponse::failure(
                    format!("Failed to parse uploaded file: {err}"),
                    None,
                ));
            }
        }
    };

    let mut guard = ctx.runtime.lock().await;
    match guard.replace_kernel_binary(&file).await {
        Ok(()) => Json(ApiResponse::success(
            String::from("sing-box core imported."),
            None,
        )),
        Err(err) => Json(ApiResponse::failure(err, None)),
    }
}

pub async fn download_latest_kernel(
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<KernelReleaseInfo>> {
    if ctx
        .kernel_download_in_progress
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        crate::route_log!(
            ctx,
            "info",
            "sing-box kernel download request skipped: another download is already in progress."
        );
        return Json(ApiResponse::failure(
            String::from("sing-box core download is in progress. Try again later."),
            None,
        ));
    }
    let _download_guard = KernelDownloadGuard {
        in_progress: ctx.kernel_download_in_progress.clone(),
        progress: ctx.kernel_download_progress.clone(),
    };
    if let Ok(mut progress) = ctx.kernel_download_progress.lock() {
        *progress = KernelDownloadProgress {
            in_progress: true,
            ..Default::default()
        };
    }

    crate::route_log!(ctx, "info", "starting latest sing-box kernel download.");
    let release = match fetch_latest_release().await {
        Ok(release) => release,
        Err(err) => {
            crate::route_log!(
                ctx,
                "warn",
                "failed to fetch latest sing-box release: {err}"
            );
            return Json(ApiResponse::failure(err, None));
        }
    };
    let version = release.tag_name.clone();
    let asset_name = kernel_asset_name(&version);
    let Some(asset) = release
        .assets
        .into_iter()
        .find(|asset| asset.name == asset_name)
    else {
        crate::route_log!(
            ctx,
            "warn",
            "sing-box release asset for the current platform was not found: {asset_name}"
        );
        return Json(ApiResponse::failure(
            format!("sing-box release asset for the current platform was not found: {asset_name}"),
            None,
        ));
    };
    update_download_progress(&ctx.kernel_download_progress, 0, Some(asset.size));
    crate::route_log!(
        ctx,
        "info",
        "sing-box release asset located: version={version} asset={}",
        asset.name
    );

    let data_dir = {
        let guard = ctx.runtime.lock().await;
        guard.singbox_host.data_dir().to_path_buf()
    };
    let archive_path = data_dir.join("tmp").join(&asset.name);
    if cached_archive_matches(&archive_path, asset.size) {
        crate::route_log!(
            ctx,
            "info",
            "reusing downloaded sing-box release asset: path={} bytes={}",
            archive_path.display(),
            asset.size
        );
    } else {
        crate::route_log!(
            ctx,
            "info",
            "starting sing-box release asset download: bytes={}",
            asset.size
        );
    }
    let archive_path = match download_release_asset(
        &asset.browser_download_url,
        archive_path,
        asset.size,
        ctx.kernel_download_progress.clone(),
    )
    .await
    {
        Ok(archive_path) => archive_path,
        Err(err) => {
            crate::route_log!(
                ctx,
                "warn",
                "failed to download sing-box release asset: {err}"
            );
            return Json(ApiResponse::failure(err, None));
        }
    };
    crate::route_log!(
        ctx,
        "info",
        "sing-box release asset prepared: path={} bytes={}",
        archive_path.display(),
        asset.size
    );
    let binary = match unpack_kernel_binary(&data_dir, &archive_path) {
        Ok(binary) => binary,
        Err(err) => {
            crate::route_log!(
                ctx,
                "warn",
                "failed to extract sing-box release asset: {err}"
            );
            return Json(ApiResponse::failure(err, None));
        }
    };
    crate::route_log!(
        ctx,
        "info",
        "sing-box executable extracted: bytes={}",
        binary.len()
    );

    let mut guard = ctx.runtime.lock().await;
    match guard.replace_kernel_binary(&binary).await {
        Ok(()) => {
            crate::route_log!(
                ctx,
                "info",
                "sing-box core downloaded and replaced: version={version}"
            );
            Json(ApiResponse::success(
                String::from("sing-box core downloaded and replaced."),
                Some(KernelReleaseInfo { version }),
            ))
        }
        Err(err) => {
            crate::route_log!(ctx, "warn", "failed to replace sing-box kernel: {err}");
            Json(ApiResponse::failure(err, None))
        }
    }
}

async fn fetch_latest_release() -> Result<GithubRelease, String> {
    github_get(GITHUB_LATEST_RELEASE_URL)
        .send()
        .await
        .map_err(|err| format!("Failed to get latest sing-box version: {err}"))?
        .error_for_status()
        .map_err(|err| format!("GitHub returned an error status: {err}"))?
        .json::<GithubRelease>()
        .await
        .map_err(|err| format!("Failed to parse GitHub release information: {err}"))
}

async fn download_release_asset(
    url: &str,
    archive_path: PathBuf,
    expected_size: u64,
    progress: Arc<Mutex<KernelDownloadProgress>>,
) -> Result<PathBuf, String> {
    if cached_archive_matches(&archive_path, expected_size) {
        update_download_progress(&progress, expected_size, Some(expected_size));
        return Ok(archive_path);
    }

    let mut response = github_get(url)
        .send()
        .await
        .map_err(|err| format!("Failed to download sing-box: {err}"))?
        .error_for_status()
        .map_err(|err| format!("GitHub download returned an error status: {err}"))?;
    let parent = archive_path
        .parent()
        .ok_or_else(|| String::from("Unable to determine sing-box download directory."))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create sing-box download directory: {err}"))?;
    let partial_path = archive_path.with_extension("part");
    let mut file = tokio::fs::File::create(&partial_path)
        .await
        .map_err(|err| format!("Failed to create sing-box download file: {err}"))?;
    let mut downloaded = 0_u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| format!("Failed to read sing-box download content: {err}"))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|err| format!("Failed to save sing-box download file: {err}"))?;
        downloaded += u64::try_from(chunk.len()).unwrap_or_default();
        update_download_progress(&progress, downloaded, Some(expected_size));
    }
    file.flush()
        .await
        .map_err(|err| format!("Failed to save sing-box download file: {err}"))?;
    if downloaded != expected_size {
        return Err(format!(
            "Downloaded sing-box file size does not match: expected={expected_size} actual={downloaded}",
        ));
    }
    if archive_path.exists() {
        fs::remove_file(&archive_path)
            .map_err(|err| format!("Failed to replace sing-box download file: {err}"))?;
    }
    fs::rename(&partial_path, &archive_path)
        .map_err(|err| format!("Failed to finalize sing-box download file: {err}"))?;
    Ok(archive_path)
}

fn update_download_progress(
    progress: &Arc<Mutex<KernelDownloadProgress>>,
    downloaded: u64,
    total: Option<u64>,
) {
    if let Ok(mut progress) = progress.lock() {
        progress.downloaded = downloaded;
        progress.total = total;
    }
}

fn cached_archive_matches(archive_path: &Path, expected_size: u64) -> bool {
    fs::metadata(archive_path)
        .map(|metadata| metadata.is_file() && metadata.len() == expected_size)
        .unwrap_or(false)
}

fn cached_latest_kernel_release() -> Option<KernelReleaseInfo> {
    let cache = LATEST_KERNEL_RELEASE_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()?;
    cache
        .as_ref()
        .filter(|cached| cached.fetched_at.elapsed() < Duration::from_secs(60 * 60))
        .map(|cached| cached.release.clone())
}

fn cache_latest_kernel_release(release: KernelReleaseInfo) {
    if let Ok(mut cache) = LATEST_KERNEL_RELEASE_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *cache = Some(CachedKernelRelease {
            release,
            fetched_at: Instant::now(),
        });
    }
}

fn github_get(url: &str) -> reqwest::RequestBuilder {
    let request = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "nsb-gui");
    match std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
    {
        Some(token) => request.bearer_auth(token),
        None => request,
    }
}

fn kernel_asset_name(version: &str) -> String {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        current => current,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "386",
        "arm" => "armv7",
        current => current,
    };
    let extension = if os == "windows" { "zip" } else { "tar.gz" };
    format!(
        "sing-box-{}-{os}-{arch}.{extension}",
        version.trim_start_matches('v')
    )
}

fn unpack_kernel_binary(data_dir: &Path, archive_path: &Path) -> Result<Vec<u8>, String> {
    let binary_name = if cfg!(windows) {
        "sing-box.exe"
    } else {
        "sing-box"
    };
    let temporary_dir = data_dir.join("tmp").join(format!(
        "nsb-sing-box-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let extract_dir = temporary_dir.join("extract");
    fs::create_dir_all(&extract_dir)
        .map_err(|err| format!("Failed to create temporary sing-box directory: {err}"))?;

    let result = (|| {
        let output = extract_archive(archive_path, &extract_dir)?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if detail.is_empty() {
                format!(
                    "Failed to extract sing-box release archive; exit status: {}",
                    output.status
                )
            } else {
                format!(
                    "Failed to extract sing-box release archive; exit status: {}: {detail}",
                    output.status
                )
            });
        }
        let binary_path = find_file(&extract_dir, binary_name).ok_or_else(|| {
            String::from("sing-box executable was not found in the release archive.")
        })?;
        fs::read(binary_path)
            .map_err(|err| format!("Failed to read extracted sing-box binary: {err}"))
    })();

    let _ = fs::remove_dir_all(temporary_dir);
    result
}

fn extract_archive(
    archive_path: &Path,
    destination: &Path,
) -> Result<std::process::Output, String> {
    #[cfg(windows)]
    let mut command = {
        let mut command = std_command("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Expand-Archive -LiteralPath $env:NSB_ARCHIVE_PATH -DestinationPath $env:NSB_EXTRACT_DIRECTORY -Force",
        ]);
        command
            .env("NSB_ARCHIVE_PATH", archive_path)
            .env("NSB_EXTRACT_DIRECTORY", destination);
        command
    };

    #[cfg(not(windows))]
    let mut command = {
        let mut command = std_command("tar");
        command
            .arg("-xzf")
            .arg(archive_path)
            .arg("-C")
            .arg(destination);
        command
    };

    command
        .output()
        .map_err(|err| format!("Failed to start extraction tool: {err}"))
}

fn find_file(directory: &Path, file_name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(directory).ok()?.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(path) = find_file(&path, file_name) {
                return Some(path);
            }
        }
    }
    None
}
