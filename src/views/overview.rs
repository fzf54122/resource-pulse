// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: overview.rs
// @Software: resource-pulse

use super::*;

impl ResourcePulse {
    pub(super) fn overview(&self, cx: &mut Context<Self>) -> Div {
        div()
            .child(self.device_summary())
            .child(
                div().flex().gap_3().mb(px(22.)).children(
                    Metric::ALL
                        .into_iter()
                        .map(|metric| self.metric_card(metric, cx)),
                ),
            )
            .child(self.cpu_preview(cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(12.))
                    .child(heading("进程速览"))
                    .child(self.control(
                        "view-processes",
                        "全部进程",
                        false,
                        |this, _| this.state.navigate(Page::Processes),
                        cx,
                    )),
            )
            .child(self.process_table(true, cx))
    }

    fn device_summary(&self) -> Div {
        let sample = &self.state.latest;
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .mb(px(18.))
            .child(heading(if self.state.ready {
                sample.os.clone()
            } else {
                "本地设备".into()
            }))
            .child(caption(if self.state.ready {
                format!(
                    "{} 个逻辑核心  ·  {} 内存  ·  已运行 {}",
                    sample.cores,
                    bytes(sample.total_memory),
                    duration(sample.uptime)
                )
            } else {
                "正在读取系统信息…".into()
            }))
    }

    fn metric_card(&self, metric: Metric, cx: &mut Context<Self>) -> pulse_ui::gpui::Stateful<Div> {
        let id = metric.id();
        let color = metric_color(metric);
        card()
            .id(id)
            .debug_selector(move || id.into())
            .flex_1()
            .min_w_0()
            .p(px(16.))
            .cursor_pointer()
            .hover(move |style| style.border_color(rgb(color)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.state.inspect_metric(metric);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(5.)).rounded_full().bg(rgb(color)))
                    .child(caption(metric.title())),
            )
            .child(
                div()
                    .mt(px(10.))
                    .text_2xl()
                    .font_weight(FontWeight::MEDIUM)
                    .child(if self.state.ready {
                        metric.value(&self.state.latest)
                    } else {
                        "—".into()
                    }),
            )
            .child(
                caption(if self.state.ready {
                    metric.detail(&self.state.latest)
                } else {
                    "正在读取…".into()
                })
                .mt(px(7.)),
            )
    }

    fn cpu_preview(&self, cx: &mut Context<Self>) -> Div {
        let trend = self.state.trend(Metric::Cpu);
        card()
            .mb(px(22.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(16.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(heading("CPU 趋势"))
                            .child(caption(format!("最近 {} 秒", self.state.range.seconds()))),
                    )
                    .child(self.control(
                        "view-history",
                        "查看历史",
                        false,
                        |this, _| this.state.inspect_metric(Metric::Cpu),
                        cx,
                    )),
            )
            .when(trend.points.is_empty(), |view| {
                view.child(caption("正在读取系统状态…").mb(px(12.)))
            })
            .child(trend_chart(&trend, Metric::Cpu, 116.))
            .child(self.time_axis())
    }
}
