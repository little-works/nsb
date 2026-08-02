use auto_launch::AutoLaunchBuilder;

const APP_NAME: &str = "NSB";

pub struct AutoLaunchHost;

impl AutoLaunchHost {
    pub fn is_enabled() -> Result<bool, String> {
        Self::auto_launch()?
            .is_enabled()
            .map_err(|err| format!("Failed to read auto-launch status: {err}"))
    }

    pub fn set_enabled(enabled: bool) -> Result<bool, String> {
        let auto_launch = Self::auto_launch()?;
        let result = if enabled {
            auto_launch.enable()
        } else {
            auto_launch.disable()
        };

        result.map_err(|err| {
            format!(
                "Failed to {} auto-launch: {err}",
                if enabled { "enable" } else { "disable" }
            )
        })?;
        auto_launch
            .is_enabled()
            .map_err(|err| format!("Failed to read auto-launch status: {err}"))
    }

    fn auto_launch() -> Result<auto_launch::AutoLaunch, String> {
        let executable =
            std::env::current_exe().map_err(|err| format!("Failed to get current executable path: {err}"))?;
        let executable = executable
            .to_str()
            .ok_or_else(|| String::from("The current executable path contains unsupported characters."))?;

        let mut builder = AutoLaunchBuilder::new();
        builder.set_app_name(APP_NAME).set_app_path(executable);

        #[cfg(target_os = "windows")]
        builder.set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);

        builder
            .build()
            .map_err(|err| format!("Failed to initialize auto-launch: {err}"))
    }
}
