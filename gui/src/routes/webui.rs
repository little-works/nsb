#[cfg(not(debug_assertions))]
use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::extract::State;
use axum::extract::{OriginalUri, Path as AxumPath};
#[cfg(not(debug_assertions))]
use axum::http::header::{CONTENT_TYPE, HeaderValue};
#[cfg(debug_assertions)]
use axum::http::header::{CONTENT_TYPE, LOCATION};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Redirect, Response};

use super::RouteState;

pub async fn index_redirect(_uri: OriginalUri) -> Response {
    Redirect::temporary("/webui/").into_response()
}

pub async fn webui_entry(
    State(ctx): State<RouteState>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    serve_webui(ctx, uri.0, headers).await
}

pub async fn webui_asset(
    State(ctx): State<RouteState>,
    AxumPath(_path): AxumPath<String>,
    uri: OriginalUri,
    headers: HeaderMap,
) -> Response {
    serve_webui(ctx, uri.0, headers).await
}

async fn serve_webui(_ctx: RouteState, uri: Uri, headers: HeaderMap) -> Response {
    #[cfg(debug_assertions)]
    {
        return proxy_to_dev_server(uri, headers).await;
    }

    #[cfg(not(debug_assertions))]
    {
        return serve_built_webui(uri, headers).await;
    }
}

#[cfg(not(debug_assertions))]
async fn serve_built_webui(uri: Uri, headers: HeaderMap) -> Response {
    let dist_dir = webui_dist_dir();
    let Some(relative_path) = webui_relative_path(uri.path()) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(resolved_path) = resolve_webui_file(&dist_dir, &relative_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    serve_file(resolved_path, &headers).await
}

#[cfg(debug_assertions)]
async fn proxy_to_dev_server(uri: Uri, headers: HeaderMap) -> Response {
    let port = std::env::var("NSB_WEBUI_PORT")
        .expect("NSB_WEBUI_PORT must be set when running the GUI in development");
    let target = format!("http://localhost:{port}{uri}");
    let client = reqwest::Client::new();

    match client.get(&target).send().await {
        Ok(response) => proxy_response(response, &headers).await,
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Html(format!(
                "<h1>WebUI dev server unavailable</h1><p>{}</p><p>Expected: <code>{}</code></p>",
                err, target
            )),
        )
            .into_response(),
    }
}

#[cfg(debug_assertions)]
async fn proxy_response(response: reqwest::Response, request_headers: &HeaderMap) -> Response {
    let status = response.status();
    let mut headers = HeaderMap::new();
    let upstream_headers = response.headers().clone();

    if let Some(location) = upstream_headers.get(LOCATION).cloned() {
        return (status, [(LOCATION, location)]).into_response();
    }

    if let Some(content_type) = upstream_headers.get(CONTENT_TYPE).cloned() {
        headers.insert(CONTENT_TYPE, content_type);
    }

    let theme = resolve_webui_theme(request_headers);
    let is_html = upstream_headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));

    match response.bytes().await {
        Ok(bytes) if is_html => match String::from_utf8(bytes.to_vec()) {
            Ok(body) => (
                status,
                headers,
                Body::from(inject_webui_placeholders(body, theme)),
            )
                .into_response(),
            Err(_) => (status, headers, Body::from(bytes)).into_response(),
        },
        Ok(bytes) => (status, headers, Body::from(bytes)).into_response(),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Html(format!("<h1>WebUI proxy read failed</h1><p>{}</p>", err)),
        )
            .into_response(),
    }
}

#[cfg(not(debug_assertions))]
async fn serve_file(path: PathBuf, request_headers: &HeaderMap) -> Response {
    let content_type = content_type_for(&path);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
            if content_type.starts_with("text/html") {
                let theme = resolve_webui_theme(request_headers);
                match String::from_utf8(bytes) {
                    Ok(body) => (headers, inject_webui_placeholders(body, theme)).into_response(),
                    Err(err) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html(format!("<h1>WebUI HTML decode failed</h1><p>{}</p>", err)),
                    )
                        .into_response(),
                }
            } else {
                (headers, bytes).into_response()
            }
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn resolve_webui_theme(headers: &HeaderMap) -> &'static str {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie| {
            cookie.split(';').find_map(|entry| {
                let mut parts = entry.trim().splitn(2, '=');
                let key = parts.next()?.trim();
                let value = parts.next()?.trim();
                (key == "nsb-theme").then_some(value)
            })
        })
        .map(validate_webui_theme)
        .unwrap_or("auto")
}

fn validate_webui_theme(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "light" => "light",
        "dark" => "dark",
        "auto" => "auto",
        _ => "auto",
    }
}

fn inject_webui_placeholders(body: String, theme: &str) -> String {
    body.replace("$data_nsb_theme", theme)
}

#[cfg(not(debug_assertions))]
fn webui_relative_path(uri_path: &str) -> Option<PathBuf> {
    let relative = uri_path
        .strip_prefix("/webui/")
        .or_else(|| uri_path.strip_prefix("/webui"))
        .unwrap_or_default();

    let normalized = if relative.is_empty() || relative.ends_with('/') {
        format!("{relative}index.html")
    } else {
        relative.to_string()
    };

    let candidate = PathBuf::from(normalized);
    if candidate.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }

    Some(candidate)
}

#[cfg(not(debug_assertions))]
fn resolve_webui_file(dist_dir: &Path, relative_path: &Path) -> Option<PathBuf> {
    let direct_path = dist_dir.join(relative_path);
    if direct_path.is_file() {
        return Some(direct_path);
    }

    let nested_index = dist_dir.join(relative_path).join("index.html");
    if nested_index.is_file() {
        return Some(nested_index);
    }

    let mut current = relative_path.to_path_buf();
    loop {
        let candidate = if current.as_os_str().is_empty() {
            dist_dir.join("index.html")
        } else {
            dist_dir.join(&current).join("index.html")
        };

        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            break;
        }
    }

    let root_index = dist_dir.join("index.html");
    root_index.is_file().then_some(root_index)
}

#[cfg(not(debug_assertions))]
fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("webmanifest") => "application/manifest+json; charset=utf-8",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("map") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(not(debug_assertions))]
fn webui_dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("webui")
        .join("dist")
        .join("client")
}
