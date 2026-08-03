use std::path::{Path, PathBuf};

use tokio::fs;

use crate::utils::path::ensure_data_dir;

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

    pub fn remote_raw_path(&self, profile_id: &str, remote_name: &str) -> PathBuf {
        self.remote_cache_dir(profile_id)
            .join(format!("{}.raw.txt", self.safe_remote_name(remote_name)))
    }

    pub async fn save_remote_raw(
        &self,
        profile_id: &str,
        remote_name: &str,
        content: &str,
    ) -> Result<(), String> {
        let path = self.remote_raw_path(profile_id, remote_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|err| {
                format!(
                    "Failed to create Remote raw cache directory: {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::write(&path, content).await.map_err(|err| {
            format!(
                "Failed to write Remote raw cache: {}: {err}",
                path.display()
            )
        })?;

        Ok(())
    }

    pub async fn read_remote_raw(
        &self,
        profile_id: &str,
        remote_name: &str,
    ) -> Result<Option<String>, String> {
        let path = self.remote_raw_path(profile_id, remote_name);
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

    fn remote_cache_dir(&self, profile_id: &str) -> PathBuf {
        self.runtime_dir().join(profile_id).join("remotes")
    }

    fn safe_remote_name(&self, remote_name: &str) -> String {
        let safe_name = remote_name
            .trim()
            .chars()
            .map(|character| {
                if character.is_control()
                    || matches!(
                        character,
                        '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                    )
                {
                    '_'
                } else {
                    character
                }
            })
            .collect::<String>()
            .trim_matches([' ', '.'])
            .to_string();
        if safe_name.is_empty() {
            String::from("default")
        } else {
            safe_name
        }
    }

    pub async fn runtime_exists(&self, id: &str) -> Result<bool, String> {
        let path = self.runtime_path(id);
        fs::try_exists(&path)
            .await
            .map_err(|err| format!("Failed to check Profile cache: {}: {err}", path.display()))
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
