use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;

use crate::app::AppAction;
use crate::routes::{SharedRuntime, build_router};

#[derive(Clone)]
pub struct ServerShutdown {
    sender: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl ServerShutdown {
    fn new(sender: oneshot::Sender<()>) -> Self {
        Self {
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

    pub fn notify(&self) {
        if let Ok(mut guard) = self.sender.try_lock()
            && let Some(sender) = guard.take()
        {
            let _ = sender.send(());
        }
    }
}

pub struct ServerHost {
    local_addr: SocketAddr,
    shutdown: ServerShutdown,
    task: JoinHandle<()>,
}

impl ServerHost {
    pub async fn start(
        runtime: SharedRuntime,
        action_proxy: tao::event_loop::EventLoopProxy<AppAction>,
        address: &str,
    ) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(address)
            .await
            .with_context(|| format!("failed to bind webui listener at {address}"))?;
        let local_addr = listener.local_addr().context("failed to read local addr")?;
        let router = build_router(runtime, action_proxy);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let task = tokio::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            let _ = server.await;
        });

        Ok(Self {
            local_addr,
            shutdown: ServerShutdown::new(shutdown_tx),
            task,
        })
    }

    pub fn shutdown(&self) -> ServerShutdown {
        self.shutdown.clone()
    }

    pub fn local_url(&self) -> String {
        format!("http://{}", self.local_addr)
    }

    pub async fn wait(self) {
        let _ = self.task.await;
    }
}
