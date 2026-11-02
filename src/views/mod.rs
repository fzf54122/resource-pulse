// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: mod.rs
// @Software: resource-pulse

mod history;
mod menubar;
mod overview;
mod processes;
mod settings;
mod shell;

use crate::{
    app::{
        CloseDetails, MinimizeWindow, OpenSettings, QuitApplication, ResourcePulse, ToggleSampling,
    },
    commands::AppCommand,
    model::{
        HistoryRange, Metric, PAGE_SIZE, Page, SamplingInterval, Sort, Trend, bytes, duration,
    },
};
use pulse_ui::{
    ChartPoint, Icon, caption, card,
    gpui::{Context, Div, FontWeight, SharedString, Window, div, prelude::*, px, rgb},
    heading, icon, line_chart, progress, section_header, status,
    theme::{self, ACCENT, BACKGROUND, BORDER, INK, MUTED, TINT},
};

fn metric_color(metric: Metric) -> u32 {
    match metric {
        Metric::Cpu => theme::CPU,
        Metric::Memory => theme::MEMORY,
        Metric::Network => theme::NETWORK,
        Metric::Disk => theme::DISK,
    }
}

fn trend_chart(trend: &Trend, metric: Metric, height: f32) -> Div {
    line_chart(
        trend
            .points
            .iter()
            .map(|point| ChartPoint {
                x: point.x,
                y: point.y,
                break_before: point.break_before,
            })
            .collect(),
        metric_color(metric),
        height,
    )
}

fn key_value(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py(px(8.))
        .child(caption(label))
        .child(div().text_sm().child(value.into()))
}

impl Render for ResourcePulse {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.state.page {
            Page::Overview => self.overview(cx),
            Page::Processes => self.processes(cx),
            Page::History => self.history(cx),
            Page::Settings => self.settings(cx),
        };
        div()
            .id("pulse-root")
            .relative()
            .key_context("Pulse")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &ToggleSampling, _, cx| {
                this.toggle_pause(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CloseDetails, _, cx| {
                this.state.selected_pid = None;
                this.menu = None;
                this.about = false;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &QuitApplication, window, cx| {
                this.execute_command(AppCommand::Quit, window, cx)
            }))
            .on_action(cx.listener(|this, _: &MinimizeWindow, window, cx| {
                this.execute_command(AppCommand::Minimize, window, cx)
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.execute_command(AppCommand::Navigate(Page::Settings), window, cx)
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                if this.menu.take().is_some() {
                    cx.notify();
                }
            }))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(INK))
            .font_family("Noto Sans CJK SC")
            .text_sm()
            .when(!self.global_menu_available, |view| {
                view.child(self.menubar(cx))
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(self.header(cx))
                            .child(
                                div()
                                    .id(("page-scroll", self.state.page as u64))
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .px(px(28.))
                                    .pb(px(24.))
                                    .when_some(self.error.clone(), |view, error| {
                                        view.child(
                                            card()
                                                .mb(px(16.))
                                                .p(px(14.))
                                                .border_color(rgb(theme::DANGER))
                                                .text_color(rgb(theme::DANGER))
                                                .child(error),
                                        )
                                    })
                                    .child(content),
                            )
                            .child(self.footer()),
                    ),
            )
            .when_some(self.menu, |view, group| {
                view.child(self.menu_popup(group, cx))
            })
            .when(self.about, |view| view.child(self.about_dialog(cx)))
    }
}

impl ResourcePulse {
    fn time_axis(&self) -> Div {
        div()
            .flex()
            .justify_between()
            .mt(px(10.))
            .child(caption(format!("−{} 秒", self.state.range.seconds())))
            .child(caption(if self.state.paused {
                "已暂停"
            } else {
                "当前"
            }))
    }
}
