use anyhow::Context;
use image::ImageFormat;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem},
};

use crate::app::AppAction;
use crate::config::AppLanguage;

pub struct TrayHost {
    _tray: TrayIcon,
    open_item: MenuItem,
    system_proxy_item: CheckMenuItem,
    data_dir_item: MenuItem,
    exit_item: MenuItem,
}

impl TrayHost {
    pub fn new(
        proxy: tao::event_loop::EventLoopProxy<AppAction>,
        app_language: AppLanguage,
        system_proxy_enabled: bool,
    ) -> anyhow::Result<Self> {
        let menu = Menu::new();
        let open_item = MenuItem::new(tray_label(app_language, TrayLabel::Open), true, None);
        let system_proxy_item = CheckMenuItem::new(
            tray_label(app_language, TrayLabel::SystemProxy),
            true,
            system_proxy_enabled,
            None,
        );
        let data_dir_item = MenuItem::new(tray_label(app_language, TrayLabel::DataDir), true, None);
        let exit_item = MenuItem::new(tray_label(app_language, TrayLabel::Exit), true, None);

        menu.append(&open_item)
            .context("failed to add Open tray menu item")?;
        menu.append(&system_proxy_item)
            .context("failed to add System Proxy tray menu item")?;
        menu.append(&data_dir_item)
            .context("failed to add Data Directory tray menu item")?;
        menu.append(&exit_item)
            .context("failed to add Exit tray menu item")?;

        register_menu_handler(
            proxy,
            open_item.id().clone(),
            system_proxy_item.id().clone(),
            data_dir_item.id().clone(),
            exit_item.id().clone(),
        );

        let tray = TrayIconBuilder::new()
            .with_tooltip("nsb")
            .with_menu(Box::new(menu))
            .with_icon(load_tray_icon()?)
            .build()
            .context("failed to create tray icon")?;

        Ok(Self {
            _tray: tray,
            open_item,
            system_proxy_item,
            data_dir_item,
            exit_item,
        })
    }

    pub fn refresh_labels(
        &mut self,
        app_language: AppLanguage,
        system_proxy_enabled: bool,
    ) -> anyhow::Result<()> {
        self.open_item
            .set_text(tray_label(app_language, TrayLabel::Open));
        self.system_proxy_item
            .set_text(tray_label(app_language, TrayLabel::SystemProxy));
        self.system_proxy_item.set_checked(system_proxy_enabled);
        self.data_dir_item
            .set_text(tray_label(app_language, TrayLabel::DataDir));
        self.exit_item
            .set_text(tray_label(app_language, TrayLabel::Exit));
        Ok(())
    }

    pub fn refresh_system_proxy(&mut self, system_proxy_enabled: bool) -> anyhow::Result<()> {
        self.system_proxy_item.set_checked(system_proxy_enabled);
        Ok(())
    }
}

enum TrayLabel {
    Open,
    SystemProxy,
    DataDir,
    Exit,
}

fn tray_label(language: AppLanguage, label: TrayLabel) -> &'static str {
    match (language, label) {
        (AppLanguage::EnUs, TrayLabel::Open) => "Open",
        (AppLanguage::ZhCn, TrayLabel::Open) => "打开",
        (AppLanguage::EnUs, TrayLabel::SystemProxy) => "System Proxy",
        (AppLanguage::ZhCn, TrayLabel::SystemProxy) => "系统代理",
        (AppLanguage::EnUs, TrayLabel::DataDir) => "Data Directory",
        (AppLanguage::ZhCn, TrayLabel::DataDir) => "数据目录",
        (AppLanguage::EnUs, TrayLabel::Exit) => "Exit",
        (AppLanguage::ZhCn, TrayLabel::Exit) => "退出",
    }
}

fn register_menu_handler(
    proxy: tao::event_loop::EventLoopProxy<AppAction>,
    open_item_id: tray_icon::menu::MenuId,
    system_proxy_item_id: tray_icon::menu::MenuId,
    data_dir_item_id: tray_icon::menu::MenuId,
    exit_item_id: tray_icon::menu::MenuId,
) {
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == open_item_id {
            Some(AppAction::OpenWebUi)
        } else if event.id == system_proxy_item_id {
            Some(AppAction::ToggleSystemProxy)
        } else if event.id == data_dir_item_id {
            Some(AppAction::OpenDataDir)
        } else if event.id == exit_item_id {
            Some(AppAction::Exit)
        } else {
            None
        };

        if let Some(action) = action {
            let _ = proxy.send_event(action);
        }
    }));
}

fn load_tray_icon() -> anyhow::Result<Icon> {
    let image = image::load_from_memory_with_format(
        include_bytes!("../../assets/nsb-logo.png"),
        ImageFormat::Png,
    )
    .context("failed to decode bundled tray icon")?
    .into_rgba8();
    let (width, height) = image.dimensions();

    Icon::from_rgba(image.into_raw(), width, height).context("failed to construct tray icon")
}
