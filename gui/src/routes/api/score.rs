use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt, future::join_all};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, sleep};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;

use crate::routes::RouteState;
use crate::state::KernelStatus;

use super::{ApiResponse, simple_response};

#[derive(Debug, Deserialize, Serialize)]
pub struct SelectProxyRequest {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProxyModeRequest {
    mode: String,
}

#[derive(Debug, Deserialize)]
pub struct LatencyTestRequest {
    proxies: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyLatencyResult {
    pub name: String,
    pub alive: bool,
    pub latency_ms: Option<u64>,
}

const LATENCY_TEST_URL: &str = "https://cp.cloudflare.com/generate_204";
const LATENCY_TEST_TIMEOUT_MS: u64 = 5_000;
const LATENCY_TEST_BATCH_SIZE: usize = 6;

struct LatencyTaskGuard {
    in_progress: Arc<AtomicBool>,
}

impl Drop for LatencyTaskGuard {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
    }
}

struct ControllerConfig {
    address: String,
    secret: String,
}

pub async fn get_proxies(State(ctx): State<RouteState>) -> Json<ApiResponse<Value>> {
    simple_response(
        async {
            let controller = controller_config(&ctx).await?;
            let url = controller.url(&["proxies"])?;
            controller.get_json(url).await
        }
        .await,
    )
}

pub async fn get_config(State(ctx): State<RouteState>) -> Json<ApiResponse<Value>> {
    simple_response(
        async {
            let controller = controller_config(&ctx).await?;
            let url = controller.url(&["configs"])?;
            controller.get_json(url).await
        }
        .await,
    )
}

pub async fn get_connections(State(ctx): State<RouteState>) -> Json<ApiResponse<Value>> {
    simple_response(
        async {
            let controller = controller_config(&ctx).await?;
            let url = controller.url(&["connections"])?;
            controller.get_json(url).await
        }
        .await,
    )
}

pub async fn set_proxy_mode(
    State(ctx): State<RouteState>,
    Json(request): Json<UpdateProxyModeRequest>,
) -> Json<ApiResponse<bool>> {
    simple_response(
        async {
            if !matches!(request.mode.as_str(), "global" | "rule" | "direct") {
                return Err(String::from("Unsupported proxy mode."));
            }

            let controller = controller_config(&ctx).await?;
            let url = controller.url(&["configs"])?;
            controller.patch_json(url, &request).await?;
            Ok(true)
        }
        .await,
    )
}

pub async fn select_proxy(
    Path(group): Path<String>,
    State(ctx): State<RouteState>,
    Json(request): Json<SelectProxyRequest>,
) -> Json<ApiResponse<bool>> {
    simple_response(
        async {
            let controller = controller_config(&ctx).await?;
            let url = controller.url(&["proxies", &group])?;
            controller.put_json(url, &request).await?;
            Ok(true)
        }
        .await,
    )
}

pub async fn start_latency_test(
    State(ctx): State<RouteState>,
    Json(request): Json<LatencyTestRequest>,
) -> Json<ApiResponse<bool>> {
    simple_response(
        async {
            let proxies = deduplicate_proxy_names(request.proxies)?;
            let cancellation = ctx.latency_cancellation.subscribe();
            if ctx
                .latency_test_in_progress
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return Ok(false);
            }

            let task_guard = LatencyTaskGuard {
                in_progress: ctx.latency_test_in_progress.clone(),
            };
            if let Err(error) = controller_config(&ctx).await {
                drop(task_guard);
                return Err(error);
            }

            tokio::spawn(run_latency_test(
                ctx.clone(),
                proxies,
                cancellation,
                task_guard,
            ));
            Ok(true)
        }
        .await,
    )
}

pub async fn stream_score(ws: WebSocketUpgrade, State(ctx): State<RouteState>) -> Response {
    ws.on_upgrade(move |socket| relay_score_stream(socket, ctx))
        .into_response()
}

pub async fn get_log_history(State(ctx): State<RouteState>) -> Json<ApiResponse<Vec<String>>> {
    simple_response(
        async {
            let log_path = {
                let guard = ctx.runtime.lock().await;
                guard.singbox_host.log_path().to_path_buf()
            };
            read_log_history(log_path).await
        }
        .await,
    )
}

pub async fn clear_logs(State(ctx): State<RouteState>) -> Json<ApiResponse<bool>> {
    simple_response(
        async {
            let log_path = {
                let guard = ctx.runtime.lock().await;
                guard.singbox_host.log_path().to_path_buf()
            };
            tokio::fs::write(&log_path, b"").await.map_err(|err| {
                format!("Failed to clear core log: {}: {err}", log_path.display())
            })?;
            Ok(true)
        }
        .await,
    )
}

async fn controller_config(ctx: &RouteState) -> Result<ControllerConfig, String> {
    let guard = ctx.runtime.lock().await;
    let kernel = &guard.controller.state.kernel;
    if kernel.status != KernelStatus::Running {
        return Err(String::from("Sing-box is not running."));
    }

    Ok(ControllerConfig {
        address: kernel.controller_addr.clone(),
        secret: kernel.controller_secret.clone(),
    })
}

fn deduplicate_proxy_names(proxies: Vec<String>) -> Result<Vec<String>, String> {
    if proxies.is_empty() {
        return Err(String::from("At least one proxy is required."));
    }

    let mut seen = HashSet::with_capacity(proxies.len());
    let mut unique = Vec::with_capacity(proxies.len());
    for proxy in proxies {
        if proxy.is_empty() {
            return Err(String::from("Proxy names must not be empty."));
        }
        if seen.insert(proxy.clone()) {
            unique.push(proxy);
        }
    }

    Ok(unique)
}

async fn run_latency_test(
    ctx: RouteState,
    proxies: Vec<String>,
    mut cancellation: broadcast::Receiver<()>,
    _task_guard: LatencyTaskGuard,
) {
    for batch in proxies.chunks(LATENCY_TEST_BATCH_SIZE) {
        let results = tokio::select! {
            biased;
            _ = cancellation.recv() => return,
            results = join_all(
                batch
                    .iter()
                    .cloned()
                    .map(|proxy| measure_proxy_latency(ctx.clone(), proxy)),
            ) => results,
        };

        if matches!(
            cancellation.try_recv(),
            Ok(()) | Err(broadcast::error::TryRecvError::Lagged(_))
        ) {
            return;
        }

        {
            let mut cache = ctx.latency_cache.lock().await;
            for result in &results {
                cache.insert(result.name.clone(), result.clone());
            }
        }

        let _ = ctx.latency_updates.send(results);
    }
}

async fn measure_proxy_latency(ctx: RouteState, proxy: String) -> ProxyLatencyResult {
    let latency_ms = async {
        let controller = controller_config(&ctx).await?;
        let payload = controller
            .get_proxy_delay(&proxy, LATENCY_TEST_URL, LATENCY_TEST_TIMEOUT_MS)
            .await?;
        first_positive_number(&payload)
            .ok_or_else(|| String::from("Controller returned no positive latency."))
    }
    .await
    .ok();

    ProxyLatencyResult {
        name: proxy,
        alive: latency_ms.is_some(),
        latency_ms,
    }
}

fn first_positive_number(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64().filter(|value| *value > 0),
        Value::Array(values) => values.iter().find_map(first_positive_number),
        Value::Object(values) => values.values().find_map(first_positive_number),
        Value::Null | Value::Bool(_) | Value::String(_) => None,
    }
}

async fn latency_snapshot(ctx: &RouteState) -> Vec<ProxyLatencyResult> {
    let cache = ctx.latency_cache.lock().await;
    let mut snapshot = cache.values().cloned().collect::<Vec<_>>();
    snapshot.sort_by(|left, right| left.name.cmp(&right.name));
    snapshot
}

impl ControllerConfig {
    fn url(&self, paths: &[&str]) -> Result<Url, String> {
        let mut url = Url::parse(&format!("http://{}/", self.address))
            .map_err(|err| format!("Invalid controller address: {err}"))?;
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| String::from("Controller address path cannot be set."))?;
        segments.pop_if_empty();
        for path in paths {
            segments.push(path);
        }
        drop(segments);
        Ok(url)
    }

    fn websocket_request(
        &self,
        paths: &[&str],
    ) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, String> {
        let mut url = self.url(paths)?;
        url.set_scheme("ws")
            .map_err(|_| String::from("Invalid controller WebSocket address."))?;
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|err| format!("Failed to create controller WebSocket request: {err}"))?;
        let authorization = HeaderValue::from_str(&format!("Bearer {}", self.secret))
            .map_err(|err| format!("Invalid controller secret: {err}"))?;
        request.headers_mut().insert(AUTHORIZATION, authorization);
        Ok(request)
    }

    async fn get_json(&self, url: Url) -> Result<Value, String> {
        let response = Client::new()
            .get(url)
            .bearer_auth(&self.secret)
            .send()
            .await
            .map_err(|err| format!("Controller request failed: {err}"))?;
        controller_json(response).await
    }

    async fn get_proxy_delay(
        &self,
        proxy: &str,
        test_url: &str,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let mut url = self.url(&["proxies", proxy, "delay"])?;
        url.query_pairs_mut()
            .append_pair("url", test_url)
            .append_pair("timeout", &timeout_ms.to_string());
        let response = Client::new()
            .get(url)
            .timeout(Duration::from_millis(timeout_ms))
            .bearer_auth(&self.secret)
            .send()
            .await
            .map_err(|err| format!("Controller delay request failed: {err}"))?;
        controller_json(response).await
    }

    async fn put_json<T: serde::Serialize>(&self, url: Url, body: &T) -> Result<(), String> {
        let response = Client::new()
            .put(url)
            .bearer_auth(&self.secret)
            .json(body)
            .send()
            .await
            .map_err(|err| format!("Controller request failed: {err}"))?;
        controller_response(response).await.map(|_| ())
    }

    async fn patch_json<T: serde::Serialize>(&self, url: Url, body: &T) -> Result<(), String> {
        let response = Client::new()
            .patch(url)
            .bearer_auth(&self.secret)
            .json(body)
            .send()
            .await
            .map_err(|err| format!("Controller request failed: {err}"))?;
        controller_response(response).await.map(|_| ())
    }
}

async fn relay_score_stream(socket: WebSocket, ctx: RouteState) {
    let mut kernel_status_receiver = {
        let guard = ctx.runtime.lock().await;
        guard.subscribe_kernel_status()
    };
    let (mut client_sender, mut client_receiver) = socket.split();
    let mut latency_receiver = ctx.latency_updates.subscribe();
    let initial_latency = latency_snapshot(&ctx).await;
    if client_sender
        .send(Message::Text(
            score_stream_message("latency_snapshot", serde_json::json!(initial_latency)).into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    let (sender, mut receiver) = mpsc::channel(64);
    let logs_task = tokio::spawn(relay_controller_websocket(
        ctx.clone(),
        &["logs"],
        "logs",
        sender.clone(),
    ));
    let traffic_task = tokio::spawn(relay_controller_websocket(
        ctx.clone(),
        &["traffic"],
        "traffic",
        sender.clone(),
    ));
    let connections_task = tokio::spawn(poll_connections(ctx.clone(), sender));

    loop {
        tokio::select! {
            client_message = client_receiver.next() => {
                if !matches!(client_message, Some(Ok(_))) {
                    break;
                }
            }
            message = receiver.recv() => {
                let Some(message) = message else {
                    break;
                };
                if client_sender.send(Message::Text(message.into())).await.is_err() {
                    break;
                }
            }
            latency = latency_receiver.recv() => {
                match latency {
                    Ok(results) => {
                        let message = score_stream_message(
                            "latency_update",
                            serde_json::json!(results),
                        );
                        if client_sender.send(Message::Text(message.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = latency_snapshot(&ctx).await;
                        let message = score_stream_message(
                            "latency_snapshot",
                            serde_json::json!(snapshot),
                        );
                        if client_sender.send(Message::Text(message.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            kernel = kernel_status_receiver.recv() => {
                match kernel {
                    Ok(kernel) => {
                        let message = serde_json::json!({
                            "type": "runtime",
                            "data": { "kernel": kernel },
                        }).to_string();
                        if client_sender.send(Message::Text(message.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    logs_task.abort();
    traffic_task.abort();
    connections_task.abort();
}

async fn relay_controller_websocket(
    ctx: RouteState,
    paths: &'static [&'static str],
    message_type: &'static str,
    sender: mpsc::Sender<String>,
) {
    loop {
        let controller = match controller_config(&ctx).await {
            Ok(controller) => controller,
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let request = match controller.websocket_request(paths) {
            Ok(request) => request,
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let (upstream, _) = match connect_async(request).await {
            Ok(connection) => connection,
            Err(_) => {
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let (mut upstream_sender, mut upstream_receiver) = upstream.split();

        while let Some(message) = upstream_receiver.next().await {
            let Ok(message) = message else {
                break;
            };
            match message {
                TungsteniteMessage::Text(text) => {
                    let payload = serde_json::from_str(&text)
                        .unwrap_or_else(|_| Value::String(text.to_string()));
                    if sender
                        .send(score_stream_message(message_type, payload))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                TungsteniteMessage::Binary(data) => {
                    let payload = serde_json::from_slice(&data)
                        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&data).into()));
                    if sender
                        .send(score_stream_message(message_type, payload))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                TungsteniteMessage::Ping(data) => {
                    if upstream_sender
                        .send(TungsteniteMessage::Pong(data))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                TungsteniteMessage::Close(_) => break,
                TungsteniteMessage::Pong(_) | TungsteniteMessage::Frame(_) => {}
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn poll_connections(ctx: RouteState, sender: mpsc::Sender<String>) {
    loop {
        if let Ok(controller) = controller_config(&ctx).await {
            if let Ok(url) = controller.url(&["connections"])
                && let Ok(payload) = controller.get_json(url).await
                && sender
                    .send(score_stream_message("connections", payload))
                    .await
                    .is_err()
            {
                return;
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
}

fn score_stream_message(message_type: &str, data: Value) -> String {
    serde_json::json!({ "type": message_type, "data": data }).to_string()
}

async fn read_log_history(path: PathBuf) -> Result<Vec<String>, String> {
    const MAX_HISTORY_BYTES: u64 = 256 * 1024;
    const MAX_HISTORY_LINES: usize = 500;

    let metadata = tokio::fs::metadata(&path).await.map_err(|err| {
        format!(
            "Failed to read core log metadata: {}: {err}",
            path.display()
        )
    })?;
    let start = metadata.len().saturating_sub(MAX_HISTORY_BYTES);
    let mut file = File::open(&path)
        .await
        .map_err(|err| format!("Failed to open core log: {}: {err}", path.display()))?;
    file.seek(SeekFrom::Start(start))
        .await
        .map_err(|err| format!("Failed to seek core log: {}: {err}", path.display()))?;

    let mut content = String::new();
    file.read_to_string(&mut content)
        .await
        .map_err(|err| format!("Failed to read core log: {}: {err}", path.display()))?;
    let content = if start > 0 {
        content.split_once('\n').map_or("", |(_, tail)| tail)
    } else {
        &content
    };

    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let start = lines.len().saturating_sub(MAX_HISTORY_LINES);
    Ok(lines.split_off(start))
}

async fn controller_json(response: reqwest::Response) -> Result<Value, String> {
    controller_response(response)
        .await?
        .json::<Value>()
        .await
        .map_err(|err| format!("Failed to parse controller response: {err}"))
}

async fn controller_response(response: reqwest::Response) -> Result<reqwest::Response, String> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    Err(format!("Controller returned {status}: {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_proxy_names_in_request_order() {
        let proxies = deduplicate_proxy_names(vec![
            String::from("alpha"),
            String::from("beta"),
            String::from("alpha"),
        ])
        .unwrap();

        assert_eq!(proxies, vec![String::from("alpha"), String::from("beta")]);
    }

    #[test]
    fn rejects_empty_proxy_names() {
        assert!(deduplicate_proxy_names(Vec::new()).is_err());
        assert!(deduplicate_proxy_names(vec![String::new()]).is_err());
    }

    #[test]
    fn finds_first_positive_latency_in_controller_payload() {
        let payload = serde_json::json!({ "ignored": 0, "delay": 123 });

        assert_eq!(first_positive_number(&payload), Some(123));
        assert_eq!(
            first_positive_number(&serde_json::json!({ "delay": 0 })),
            None
        );
    }

    #[test]
    fn serializes_latency_result_for_websocket_contract() {
        let result = ProxyLatencyResult {
            name: String::from("alpha"),
            alive: false,
            latency_ms: None,
        };

        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "name": "alpha",
                "alive": false,
                "latencyMs": null,
            })
        );
    }

    #[test]
    fn latency_task_guard_releases_global_lock() {
        let in_progress = Arc::new(AtomicBool::new(true));
        {
            let _guard = LatencyTaskGuard {
                in_progress: in_progress.clone(),
            };
        }

        assert!(!in_progress.load(Ordering::Acquire));
    }
}
