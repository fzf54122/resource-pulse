// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: shell.rs
// @Software: resource-pulse

use super::*;

impl ResourcePulse {
    pub(super) fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .w(px(184.))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .px(px(12.))
            .py(px(24.))
            .bg(rgb(theme::SIDEBAR))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px(px(12.))
                    .mb(px(34.))
                    .child(pulse_ui::gpui::img(crate::brand::mascot()).size(px(28.)))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Pulse"),
                    ),
            )
            .child(caption("监控").px(px(12.)).mb(px(10.)))
            .children(Page::ALL.into_iter().map(|page| {
                let (id, glyph) = match page {
                    Page::Overview => ("nav-overview", Icon::Overview),
                    Page::Processes => ("nav-processes", Icon::Processes),
                    Page::History => ("nav-history", Icon::History),
                    Page::Settings => ("nav-settings", Icon::Settings),
                };
                let selected = self.state.page == page;
                div()
                    .id(id)
                    .debug_selector(move || id.into())
                    .flex()
                    .items_center()
                    .gap_3()
                    .px(px(12.))
                    .py(px(11.))
                    .mb(px(4.))
                    .rounded(px(4.))
                    .text_color(rgb(if selected { ACCENT } else { MUTED }))
                    .when(selected, |view| {
                        view.bg(rgb(TINT)).font_weight(FontWeight::MEDIUM)
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.navigate(page);
                        cx.notify();
                    }))
                    .child(icon(glyph, if selected { ACCENT } else { MUTED }))
                    .child(page.title())
            }))
            .child(div().flex_1())
            .child(
                div()
                    .px(px(12.))
                    .pt(px(16.))
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .child(caption("本地设备"))
                    .child(div().text_xs().mt(px(6.)).child(if self.state.ready {
                        self.state.latest.os.clone()
                    } else {
                        std::env::consts::OS.into()
                    }))
                    .child(caption(format!("v{}", env!("CARGO_PKG_VERSION"))).mt(px(16.))),
            )
    }

    pub(super) fn header(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px(px(28.))
            .py(px(22.))
            .flex_shrink_0()
            .child(
                div()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.state.page.title()),
                    )
                    .child(caption(self.state.page.description()).mt(px(5.))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(status(self.sampling_status(), self.status_color()))
                    .child(
                        self.control(
                            "pause",
                            if self.state.paused {
                                "继续监控"
                            } else {
                                "暂停监控"
                            },
                            self.state.paused,
                            |this, cx| this.toggle_pause(cx),
                            cx,
                        )
                        .when(self.error.is_some(), |view| {
                            view.opacity(0.4).cursor_default()
                        }),
                    ),
            )
    }

    fn status_color(&self) -> u32 {
        if self.error.is_some() {
            theme::DANGER
        } else if self.state.paused {
            theme::WARNING
        } else if self.state.ready {
            theme::SUCCESS
        } else {
            MUTED
        }
    }

    pub(super) fn footer(&self) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px(px(28.))
            .py(px(9.))
            .border_t_1()
            .border_color(rgb(BORDER))
            .bg(rgb(theme::SURFACE))
            .flex_shrink_0()
            .child(caption(format!(
                "每 {} 秒更新",
                self.state.interval.seconds(),
            )))
            .child(caption(if self.state.paused {
                "已暂停更新".into()
            } else if self.error.is_some() {
                "更新暂不可用".into()
            } else {
                format!(
                    "运行 {}  ·  空格键暂停或继续",
                    duration(self.state.latest.elapsed)
                )
            }))
    }
}
