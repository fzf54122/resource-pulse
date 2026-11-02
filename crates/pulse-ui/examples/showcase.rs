// @Time    : 2026/10/30 18:04
// @Author  : fzf
// @FileName: showcase.rs
// @Software: resource-pulse

use pulse_ui::{
    application, button, caption, card,
    gpui::{App, Bounds, Context, Window, WindowBounds, WindowOptions, prelude::*, px, size},
};

struct Showcase {
    count: usize,
}

impl Render for Showcase {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        card()
            .size_full()
            .child(caption("独立组件示例，不加载任何监控业务代码。"))
            .child(
                button(
                    "increment",
                    format!("已点击 {} 次", self.count),
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.count += 1;
                        cx.notify();
                    }),
                )
                .mt(px(20.)),
            )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        cx.on_window_closed(|cx, _window_id| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(480.), px(280.)), cx);
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Showcase { count: 0 }),
        ) {
            eprintln!("无法打开组件示例：{error}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
