#![windows_subsystem = "windows"]

mod app;
mod config;
mod hosts;
mod logger;
mod routes;
mod state;
mod utils;

use app::{AppAction, DesktopApp};
use tao::event_loop::EventLoopBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logger::init();

    let event_loop = EventLoopBuilder::<AppAction>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let app = DesktopApp::new(proxy);

    app.run(event_loop).await
}
