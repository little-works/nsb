use std::path::PathBuf;

pub fn ensure_data_dir(sub_dir: &str) -> Result<PathBuf, String> {
    let path = resolve_data_dir(sub_dir)?;

    std::fs::create_dir_all(&path)
        .map_err(|err| format!("Failed to create data directory: {}: {err}", path.display()))?;

    Ok(path)
}

pub fn ensure_config_dir(sub_dir: &str) -> Result<PathBuf, String> {
    let path = resolve_config_dir(sub_dir)?;

    std::fs::create_dir_all(&path).map_err(|err| {
        format!(
            "Failed to create configuration directory: {}: {err}",
            path.display()
        )
    })?;

    Ok(path)
}

fn resolve_data_dir(sub_dir: &str) -> Result<PathBuf, String> {
    let root_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dev-data")
    } else {
        detect_home_dir()?.join(".nsb")
    };

    Ok(append_sub_dir(root_dir, sub_dir))
}

fn resolve_config_dir(sub_dir: &str) -> Result<PathBuf, String> {
    let root_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dev-config")
    } else {
        detect_home_dir()?.join(".config").join("nsb")
    };

    Ok(append_sub_dir(root_dir, sub_dir))
}

fn append_sub_dir(root_dir: PathBuf, sub_dir: &str) -> PathBuf {
    if sub_dir.trim().is_empty() {
        root_dir
    } else {
        root_dir.join(sub_dir)
    }
}

fn detect_home_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .ok_or_else(|| String::from("User home directory was not found."))
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| String::from("User home directory was not found."))
    }
}
