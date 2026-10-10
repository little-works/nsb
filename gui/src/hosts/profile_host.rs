use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::fs;

use crate::utils::path::ensure_data_dir;

static REMOTE_CACHE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct ProfileHost {
    data_dir: PathBuf,
}

impl ProfileHost {
    pub fn detect() -> Result<Self, String> {
        Ok(Self {
            data_dir: ensure_data_dir("")?,
        })
    }

    pub fn runtime_path(&self, id: &str) -> PathBuf {
        self.runtime_dir().join(format!("{id}.json"))
    }

    pub async fn save_runtime(&self, id: &str, content: &str) -> Result<(), String> {
        let path = self.runtime_path(id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                format!(
                    "Failed to create Profile cache directory: {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::write(&path, content)
            .await
            .map_err(|err| format!("Failed to write Profile cache: {}: {err}", path.display()))
    }

    pub async fn read_runtime(&self, id: &str) -> Result<String, String> {
        let path = self.runtime_path(id);
        fs::read_to_string(&path)
            .await
            .map_err(|err| format!("Failed to read Profile cache: {}: {err}", path.display()))
    }

    pub async fn delete_runtime(&self, id: &str) -> Result<(), String> {
        let path = self.runtime_path(id);
        self.remove_cache_path_if_exists(&path).await?;

        let legacy_path = self.legacy_runtime_path(id);
        if legacy_path != path {
            self.remove_cache_path_if_exists(&legacy_path).await?;
        }

        Ok(())
    }

    pub fn remote_raw_path(&self, url: &str) -> PathBuf {
        self.remote_cache_dir().join(format!("{}.raw.txt", sha256_hex(url)))
    }

    pub async fn save_remote_raw(&self, url: &str, content: &str) -> Result<(), String> {
        let path = self.remote_raw_path(url);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                format!(
                    "Failed to create Remote raw cache directory: {}: {err}",
                    parent.display()
                )
            })?;
        }
        let temp_path = self.remote_raw_temp_path(&path);
        fs::write(&temp_path, content).await.map_err(|err| {
            format!(
                "Failed to write Remote raw cache: {}: {err}",
                temp_path.display()
            )
        })?;
        if let Err(err) = atomic_replace(&temp_path, &path).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(format!(
                "Failed to replace Remote raw cache: {}: {err}",
                path.display()
            ));
        }

        Ok(())
    }

    pub async fn read_remote_raw(&self, url: &str) -> Result<Option<String>, String> {
        let path = self.remote_raw_path(url);
        if !fs::try_exists(&path).await.map_err(|err| {
            format!(
                "Failed to check Remote raw cache: {}: {err}",
                path.display()
            )
        })? {
            return Ok(None);
        }
        fs::read_to_string(&path)
            .await
            .map(Some)
            .map_err(|err| format!("Failed to read Remote raw cache: {}: {err}", path.display()))
    }

    fn remote_cache_dir(&self) -> PathBuf {
        self.data_dir.join("runtime").join("remotes")
    }

    fn remote_raw_temp_path(&self, path: &Path) -> PathBuf {
        let counter = REMOTE_CACHE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("remote.raw.txt");
        path.with_file_name(format!(".{file_name}.{}.{}.tmp", std::process::id(), counter))
    }

    pub async fn migrate_runtime(&self, id: &str) -> Result<(), String> {
        let path = self.runtime_path(id);
        if fs::try_exists(&path)
            .await
            .map_err(|err| format!("Failed to check Profile cache: {}: {err}", path.display()))?
        {
            return Ok(());
        }

        let legacy_path = self.legacy_runtime_path(id);
        let metadata = match fs::symlink_metadata(&legacy_path).await {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => {
                return Err(format!(
                    "Failed to check Profile cache: {}: {err}",
                    legacy_path.display()
                ));
            }
        };
        if !metadata.file_type().is_file() {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                format!(
                    "Failed to create Profile cache directory: {}: {err}",
                    parent.display()
                )
            })?;
        }

        fs::rename(&legacy_path, &path).await.map_err(|err| {
            format!(
                "Failed to migrate Profile cache: {} -> {}: {err}",
                legacy_path.display(),
                path.display()
            )
        })
    }

    fn runtime_dir(&self) -> PathBuf {
        self.data_dir.join("runtime").join("profiles")
    }

    fn legacy_runtime_path(&self, id: &str) -> PathBuf {
        self.runtime_dir().join(id)
    }

    async fn remove_cache_path_if_exists(&self, path: &Path) -> Result<(), String> {
        let metadata = match fs::symlink_metadata(path).await {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => {
                return Err(format!(
                    "Failed to check Profile cache: {}: {err}",
                    path.display()
                ));
            }
        };

        if metadata.file_type().is_dir() {
            fs::remove_dir_all(path).await.map_err(|err| {
                format!(
                    "Failed to delete Profile cache directory: {}: {err}",
                    path.display()
                )
            })
        } else {
            fs::remove_file(path)
                .await
                .map_err(|err| format!("Failed to delete Profile cache: {}: {err}", path.display()))
        }
    }
}

fn sha256_hex(value: &str) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, value.as_bytes());
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn atomic_replace(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    #[cfg(windows)]
    {
        let source = source.to_path_buf();
        let destination = destination.to_path_buf();
        tokio::task::spawn_blocking(move || atomic_replace_windows(&source, &destination))
            .await
            .map_err(std::io::Error::other)?
    }

    #[cfg(not(windows))]
    {
        fs::rename(source, destination).await
    }
}

#[cfg(windows)]
fn atomic_replace_windows(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
