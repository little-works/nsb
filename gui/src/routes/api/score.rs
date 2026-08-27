use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::mpsc;
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
pub struct ProxyDelayQuery {
    url: String,
    timeout: u64,
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

pub async fn get_proxy_delay(
    Path(proxy): Path<String>,
    Query(query): Query<ProxyDelayQuery>,
    State(ctx): State<RouteState>,
) -> Json<ApiResponse<Value>> {
    simple_response(
        async {
            let controller = controller_config(&ctx).await?;
            let mut url = controller.url(&["proxies", &proxy, "delay"])?;
            url.query_pairs_mut()
                .append_pair("url", &query.url)
                .append_pair("timeout", &query.timeout.to_string());
            controller.get_json(url).await
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
    let connections_task = tokio::spawn(poll_connections(ctx, sender));

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
