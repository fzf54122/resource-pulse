// @Time    : 2026/10/30 18:00
// @Author  : fzf
// @FileName: main.rs
// @Software: Rust

mod app;
mod brand;
mod collector;
mod commands;
mod desktop;
mod menu;
mod model;
mod preferences;
mod views;

use app::ResourcePulse;
use pulse_ui::{
    application,
    gpui::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size},
};

fn main() {
    let (commands, receiver) = async_channel::bounded(32);
    let instance = match desktop::Instance::acquire(commands.clone()) {
        Ok(Some(instance)) => Some(instance),
        Ok(None) => return,
        Err(error) => {
            eprintln!("Pulse 单实例服务不可用：{error}");
            None
        }
    };
    application().run(move |cx: &mut App| {
        cx.set_app_identity(brand::APP_ID, "Pulse");
        app::bind_keys(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1220.), px(850.)), cx);
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.), px(640.))),
                app_id: Some(brand::APP_ID.into()),
                icon: brand::window_icon()
                    .map_err(|error| eprintln!("Pulse 图标加载失败：{error}"))
                    .ok(),
                titlebar: Some(TitlebarOptions {
                    title: Some("Pulse · 系统监控".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let mut app = ResourcePulse::new(cx);
                    app.focus.focus(window, cx);
                    app.start(window, cx);
                    app.attach_desktop(commands, receiver, instance, window, cx);
                    app
                })
            },
        ) {
            eprintln!("无法打开 Pulse 窗口：{error}。请检查桌面会话和 GPU/Vulkan 驱动。");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests;
