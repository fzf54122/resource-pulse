// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: settings.rs
// @Software: resource-pulse

use super::*;

fn setting_row(title: &'static str, description: &'static str, controls: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .p(px(20.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(heading(title))
                .child(caption(description).mt(px(6.))),
        )
        .child(controls)
}

impl ResourcePulse {
    pub(super) fn settings(&self, cx: &mut Context<Self>) -> Div {
        div()
            .when_some(self.preference_error, |view, error| {
                view.child(caption(error).text_color(rgb(theme::WARNING)).mb(px(16.)))
            })
            .child(section_header("常规", ""))
            .child(
                card()
                    .p_0()
                    .child(setting_row(
                        "更新频率",
                        "选择资源数据的更新间隔。",
                        div()
                            .flex()
                            .gap_2()
                            .children(SamplingInterval::ALL.into_iter().map(|interval| {
                                self.control(
                                    interval.id(),
                                    format!("{} 秒", interval.seconds()),
                                    self.state.interval == interval,
                                    move |this, _| this.set_interval(interval),
                                    cx,
                                )
                            })),
                    ))
                    .child(
                        setting_row(
                            "关闭窗口时",
                            "托盘运行时，可随时点击图标返回。",
                            div()
                                .flex()
                                .gap_2()
                                .child(self.control(
                                    "close-to-tray",
                                    "保留在托盘",
                                    self.preferences.close_to_tray,
                                    |this, _| this.set_close_to_tray(true),
                                    cx,
                                ))
                                .child(self.control(
                                    "close-to-quit",
                                    "退出应用",
                                    !self.preferences.close_to_tray,
                                    |this, _| this.set_close_to_tray(false),
                                    cx,
                                )),
                        )
                        .border_t_1()
                        .border_color(rgb(BORDER)),
                    ),
            )
            .when_some(self.desktop_error.clone(), |view, error| {
                view.child(caption(error).mt(px(10.)))
            })
            .child(section_header("监控", "").mt(px(28.)))
            .child(
                card().p_0().child(setting_row(
                    "实时更新",
                    "暂停后仍可查看已有的资源记录。",
                    self.control(
                        "settings-pause",
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
                )),
            )
            .child(section_header("隐私", "").mt(px(28.)))
            .child(
                card()
                    .child(heading("仅在本机运行"))
                    .child(caption("所有监控数据仅在本机处理，不会上传。").mt(px(8.)))
                    .child(caption("Pulse 不会更改系统设置或终止其他进程。").mt(px(6.))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mt(px(28.))
                    .child(caption(format!("Pulse {}", env!("CARGO_PKG_VERSION"))))
                    .child(self.control(
                        "settings-about",
                        "关于 Pulse",
                        false,
                        |this, _| this.about = true,
                        cx,
                    )),
            )
    }
}
