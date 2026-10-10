use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use tokio::sync::{Mutex, watch};

use crate::state::ProfileHeader;

const PROFILE_USER_AGENT: &str = concat!(
    "ClashforWindows/0.20.16 clash-verge/2.5.2 NSB/",
    env!("CARGO_PKG_VERSION"),
);
const REMOTE_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

type DownloadResult = Result<String, String>;

#[derive(Clone, Default)]
pub(crate) struct RemoteDownloadCoordinator {
    in_flight: Arc<Mutex<HashMap<String, watch::Sender<Option<DownloadResult>>>>>,
}

impl RemoteDownloadCoordinator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn download(
        &self,
        url: &str,
        headers: &[ProfileHeader],
    ) -> DownloadResult {
        let (mut receiver, should_spawn) = {
            let mut in_flight = self.in_flight.lock().await;
            if let Some(sender) = in_flight.get(url) {
                (sender.subscribe(), false)
            } else {
                let (sender, receiver) = watch::channel(None);
                in_flight.insert(url.to_string(), sender);
                (receiver, true)
            }
        };

        if should_spawn {
            let coordinator = self.clone();
            let url = url.to_string();
            let headers = headers.to_vec();
            tokio::spawn(async move {
                let result = match AssertUnwindSafe(download_profile(&url, &headers))
                    .catch_unwind()
                    .await
                {
                    Ok(result) => result,
                    Err(_) => Err(String::from("Remote download worker panicked.")),
                };

                let mut in_flight = coordinator.in_flight.lock().await;
                if let Some(sender) = in_flight.remove(&url) {
                    let _ = sender.send(Some(result));
                }
            });
        }

        loop {
            if let Some(result) = receiver.borrow_and_update().clone() {
                return result;
            }
            receiver
                .changed()
                .await
                .map_err(|_| String::from("Remote download worker stopped unexpectedly."))?;
        }
    }
}

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
