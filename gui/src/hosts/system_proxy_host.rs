#[cfg(windows)]
use crate::utils::command::std_command;

pub struct SystemProxyHost;

impl SystemProxyHost {
    pub fn configure(enabled: bool, mixed_port: u16) -> Result<(), String> {
        #[cfg(windows)]
        {
            const INTERNET_SETTINGS: &str =
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
            let server = if enabled {
                format!("127.0.0.1:{mixed_port}")
            } else {
                String::new()
            };

            run_reg_add(
                INTERNET_SETTINGS,
                "ProxyEnable",
                "REG_DWORD",
                if enabled { "1" } else { "0" },
            )?;
            run_reg_add(INTERNET_SETTINGS, "ProxyServer", "REG_SZ", &server)?;
            run_reg_add(
                INTERNET_SETTINGS,
                "ProxyOverride",
                "REG_SZ",
                "<local>;localhost;127.*;[::1]",
            )?;
            return Ok(());
        }

        #[cfg(not(windows))]
        {
            if enabled {
                return Err(String::from("Configuring the system proxy is not supported on this platform."));
            }
            let _ = mixed_port;
            Ok(())
        }
    }
}

#[cfg(windows)]
fn run_reg_add(key: &str, name: &str, value_type: &str, value: &str) -> Result<(), String> {
    let output = std_command("reg")
        .args(["add", key, "/v", name, "/t", value_type, "/d", value, "/f"])
        .output()
        .map_err(|err| format!("Failed to run Windows Registry command: {err}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "Failed to configure Windows system proxy: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}
