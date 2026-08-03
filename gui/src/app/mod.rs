mod controller;
mod runtime;

use std::sync::Arc;

use anyhow::Context;
use log::info;
use tao::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use tokio::sync::Mutex;

use crate::config::AppLanguage;
use crate::hosts::{ServerHost, TrayHost};

pub use controller::AppSnapshot;
pub use runtime::{GuiRuntime, SharedGuiRuntime, update_profile_runtime};

#[derive(Debug, Clone)]
pub enum AppAction {
    OpenWebUi,
    OpenDataDir,
    ToggleSystemProxy,
    RefreshTraySystemProxy(bool),
    RefreshTrayLabels {
        app_language: AppLanguage,
        system_proxy_enabled: bool,
    },
    Exit,
}

pub struct DesktopApp {
    proxy: EventLoopProxy<AppAction>,
    runtime: Option<SharedGuiRuntime>,
    server_host: Option<ServerHost>,
    tray_host: Option<TrayHost>,
}

impl DesktopApp {
    pub fn new(proxy: EventLoopProxy<AppAction>) -> Self {
        Self {
            proxy,
            runtime: None,
            server_host: None,
            tray_host: None,
        }
    }

    pub fn proxy(&self) -> EventLoopProxy<AppAction> {
        self.proxy.clone()
    }

    pub async fn run(mut self, event_loop: EventLoop<AppAction>) -> anyhow::Result<()> {
        let runtime = Arc::new(Mutex::new(
            GuiRuntime::detect().await.map_err(anyhow::Error::msg)?,
        ));

        let preferred_port = {
            let guard = runtime.lock().await;
            guard.controller.state.gui_config.app_port
        };
        let webui_addr =
            allocate_webui_addr(preferred_port).context("failed to allocate webui port")?;
        let server_host = ServerHost::start(runtime.clone(), self.proxy(), &webui_addr)
            .await
            .context("failed to start axum webui")?;

        let data_dir = {
            let guard = runtime.lock().await;
            guard.singbox_host.data_dir().to_path_buf()
        };
        let tray_config = {
            let guard = runtime.lock().await;
            (
                guard.controller.state.gui_config.app_language,
                guard.controller.state.gui_config.system_proxy_enabled,
            )
        };
        let webui_url = server_host.local_url();
        let shutdown = server_host.shutdown();

        self.runtime = Some(runtime.clone());
        self.server_host = Some(server_host);

        self.tray_host = Some(
            TrayHost::new(self.proxy(), tray_config.0, tray_config.1)
                .context("failed to initialize tray host")?,
        );
        spawn_auto_start_kernel(runtime.clone());
        spawn_profile_scheduler(self.runtime.as_ref().expect("runtime initialized").clone());

        info!("desktop app ready, webui={webui_url}");

        let runtime_for_events = runtime.clone();
        let action_proxy = self.proxy();
        let mut tray_host = self.tray_host.take().expect("tray host initialized");
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            if let tao::event::Event::UserEvent(action) = event {
                match action {
                    AppAction::OpenWebUi => {
                        let _ = webbrowser::open(&webui_url);
                    }
                    AppAction::OpenDataDir => {
                        let _ = open::that(&data_dir);
                    }
                    AppAction::ToggleSystemProxy => {
                        let runtime = runtime_for_events.clone();
                        let action_proxy = action_proxy.clone();
                        tokio::spawn(async move {
                            let result = {
                                let mut guard = runtime.lock().await;
                                let config = &guard.controller.state.gui_config;
                                let mixed_port = config.mixed_port;
                                let app_port = config.app_port;
                                let allow_lan = config.allow_lan;
                                let system_proxy_enabled = !config.system_proxy_enabled;
                                guard
                                    .save_runtime_settings(
                                        mixed_port,
                                        app_port,
                                        allow_lan,
                                        system_proxy_enabled,
                                    )
                                    .await
                                    .map(|()| system_proxy_enabled)
                            };
                            match result {
                                Ok(system_proxy_enabled) => {
                                    let _ = action_proxy.send_event(
                                        AppAction::RefreshTraySystemProxy(system_proxy_enabled),
                                    );
                                }
                                Err(err) => log::warn!("tray system proxy toggle failed: {err}"),
                            }
                        });
                    }
                    AppAction::RefreshTraySystemProxy(system_proxy_enabled) => {
                        if let Err(err) = tray_host.refresh_system_proxy(system_proxy_enabled) {
                            log::warn!("failed to refresh tray system proxy state: {err}");
                        }
                    }
                    AppAction::RefreshTrayLabels {
                        app_language,
                        system_proxy_enabled,
                    } => {
                        if let Err(err) =
                            tray_host.refresh_labels(app_language, system_proxy_enabled)
                        {
                            log::warn!("failed to refresh tray labels: {err}");
                        }
                    }
                    AppAction::Exit => {
                        shutdown.clone().notify();
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
        });

        #[allow(unreachable_code)]
        {
            if let Some(runtime) = self.runtime.as_ref() {
                let mut guard = runtime.lock().await;
                let _ = guard.shutdown_kernel_on_exit().await;
            }

            if let Some(server_host) = self.server_host.take() {
                server_host.shutdown().notify();
                server_host.wait().await;
            }

            Ok(())
        }
    }
}

fn allocate_webui_addr(preferred_port: u16) -> anyhow::Result<String> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", preferred_port))
        .or_else(|_| std::net::TcpListener::bind("127.0.0.1:0"))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(format!("127.0.0.1:{port}"))
}

fn spawn_auto_start_kernel(runtime: SharedGuiRuntime) {
    tokio::spawn(async move {
        let mut guard = runtime.lock().await;
        if let Err(err) = guard.auto_start_kernel_if_needed().await {
            log::warn!("auto start kernel failed: {err}");
        }
    });
}

fn spawn_profile_scheduler(runtime: SharedGuiRuntime) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            for profile_id in runtime::due_profile_ids(runtime.clone()).await {
                let runtime = runtime.clone();
                tokio::spawn(async move {
                    if let Err(err) =
                        runtime::update_profile_runtime(runtime, profile_id, true).await
                    {
                        log::warn!("automatic Profile update failed: {err}");
                    }
                });
            }
        }
    });
}
