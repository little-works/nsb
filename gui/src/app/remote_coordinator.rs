use std::time::Duration;

use crate::state::ProfileHeader;

const PROFILE_USER_AGENT: &str = concat!(
    "ClashforWindows/0.20.16 clash-verge/2.5.2 NSB/",
    env!("CARGO_PKG_VERSION"),
);
const REMOTE_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

type DownloadResult = Result<String, String>;

pub(crate) async fn download_profile(
    url: &str,
    headers: &[ProfileHeader],
) -> DownloadResult {
    let mut request = reqwest::Client::new()
        .get(url)
        .timeout(REMOTE_REQUEST_TIMEOUT)
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
