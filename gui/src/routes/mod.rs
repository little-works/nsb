pub mod api;
mod webui;

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use log::info;
use tokio::sync::Mutex;

use crate::app::{AppAction, GuiRuntime};

pub type SharedRuntime = Arc<Mutex<GuiRuntime>>;

static TRACE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

tokio::task_local! {
    static REQUEST_TRACE_ID: String;
}

#[macro_export]
macro_rules! route_log {
    ($ctx:expr, $level:expr, $($arg:tt)*) => {{
        let _ = &$ctx;
        $crate::routes::log_with_trace($level, format_args!($($arg)*))
    }};
}

#[derive(Clone, Debug)]
pub struct RequestTraceId(pub String);

#[derive(Clone)]
pub struct RouteState {
    pub runtime: SharedRuntime,
    pub app_action_proxy: tao::event_loop::EventLoopProxy<AppAction>,
    pub kernel_download_in_progress: Arc<AtomicBool>,
}

pub fn build_router(
    runtime: SharedRuntime,
    app_action_proxy: tao::event_loop::EventLoopProxy<AppAction>,
) -> Router {
    Router::new()
        .route("/", get(webui::index_redirect))
        .route("/webui", get(webui::webui_entry))
        .route("/webui/", get(webui::webui_entry))
        .route("/webui/{*path}", get(webui::webui_asset))
        .route("/api/runtime", get(api::kernel::get_runtime))
        .route("/api/kernel/version", get(api::kernel::get_kernel_version))
        .route(
            "/api/kernel/latest",
            get(api::kernel::get_latest_kernel_release),
        )
        .route("/api/kernel/toggle", post(api::kernel::toggle_kernel))
        .route(
            "/api/kernel/import",
            post(api::kernel::import_kernel_binary).layer(DefaultBodyLimit::max(128 * 1024 * 1024)),
        )
        .route(
            "/api/kernel/download",
            post(api::kernel::download_latest_kernel),
        )
        .route("/api/score/proxies", get(api::score::get_proxies))
        .route("/api/score/connections", get(api::score::get_connections))
        .route(
            "/api/score/configs",
            get(api::score::get_config).patch(api::score::set_proxy_mode),
        )
        .route("/api/score/proxies/{group}", put(api::score::select_proxy))
        .route(
            "/api/score/proxies/{proxy}/delay",
            get(api::score::get_proxy_delay),
        )
        .route("/api/score/stream", get(api::score::stream_score))
        .route("/api/score/logs/history", get(api::score::get_log_history))
        .route("/api/score/logs", post(api::score::clear_logs))
        .route("/api/profiles", get(api::profiles::list_profiles))
        .route("/api/profiles/{id}", get(api::profiles::get_profile))
        .route("/api/profiles", post(api::profiles::create_profile))
        .route("/api/profiles/import", post(api::profiles::import_profile))
        .route(
            "/api/profiles/{id}/activate",
            post(api::profiles::set_current_profile),
        )
        .route(
            "/api/profiles/{id}/content",
            get(api::profiles::get_profile_content).put(api::profiles::save_profile_content),
        )
        .route("/api/profiles/{id}", put(api::profiles::update_profile))
        .route("/api/profiles/{id}", delete(api::profiles::delete_profile))
        .route(
            "/api/profiles/{id}/refresh",
            post(api::profiles::refresh_profile_by_id),
        )
        .route(
            "/api/profiles/refresh",
            post(api::profiles::refresh_profile),
        )
        .route(
            "/api/settings",
            get(api::settings::get_settings).post(api::settings::save_settings),
        )
        .route(
            "/api/settings/app-language",
            post(api::settings::save_app_language),
        )
        .route(
            "/api/settings/auto-launch",
            get(api::settings::get_auto_launch).post(api::settings::update_auto_launch),
        )
        .layer(middleware::from_fn(access_log))
        .with_state(RouteState {
            runtime,
            app_action_proxy,
            kernel_download_in_progress: Arc::new(AtomicBool::new(false)),
        })
}

async fn access_log(request: Request, next: Next) -> Response {
    let trace_context = RequestTraceId(generate_trace_id());
    let trace_id = trace_context.0.clone();
    let method = request.method().clone();
    let url = request.uri().to_string();
    let started_at = Instant::now();
    let mut request = request;
    request.extensions_mut().insert(trace_context);

    info!("access started trace_id={trace_id} method={method} url={url}");
    let response = REQUEST_TRACE_ID
        .scope(trace_id.clone(), next.run(request))
        .await;

    info!(
        "access completed trace_id={trace_id} status={} duration_ms={}",
        response.status().as_u16(),
        started_at.elapsed().as_millis(),
    );

    response
}

fn generate_trace_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = TRACE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{timestamp:x}-{sequence:x}")
}

pub fn log_with_trace(level: &str, message: impl std::fmt::Display) {
    let trace_id = REQUEST_TRACE_ID
        .try_with(|trace_id| trace_id.clone())
        .unwrap_or_else(|_| String::from("no-trace"));

    match level {
        "info" => info!("trace_id={trace_id} {message}"),
        "warn" => log::warn!("trace_id={trace_id} {message}"),
        _ => unreachable!("unsupported log level"),
    }
}
