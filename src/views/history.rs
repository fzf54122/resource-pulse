// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: history.rs
// @Software: resource-pulse

use super::*;

impl ResourcePulse {
    pub(super) fn history(&self, cx: &mut Context<Self>) -> Div {
        let metric = self.state.metric;
        let trend = self.state.trend(metric);
        div()
            .child(self.history_toolbar(cx))
            .child(self.history_chart(metric, &trend))
            .child(caption(metric.note()).mt(px(14.)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .mt(px(26.))
                    .child(
                        div().child(heading("历史记录")).child(
                            caption(format!(
                                "{} 条记录 · 退出应用后清除",
                                self.state.history.len(),
                            ))
                            .mt(px(6.)),
                        ),
                    )
                    .child(self.control(
                        "clear-history",
                        "清空历史",
                        false,
                        |this, _| this.state.history.clear(),
                        cx,
                    )),
            )
    }

    fn history_toolbar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .mb(px(18.))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children(Metric::ALL.into_iter().map(|metric| {
                        self.control(
                            metric.tab_id(),
                            metric.title(),
                            self.state.metric == metric,
                            move |this, _| this.state.metric = metric,
                            cx,
                        )
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children(HistoryRange::ALL.into_iter().map(|range| {
                        self.control(
                            range.id(),
                            format!("{} 秒", range.seconds()),
                            self.state.range == range,
                            move |this, _| this.state.range = range,
                            cx,
                        )
                    })),
            )
    }

    fn history_chart(&self, metric: Metric, trend: &Trend) -> Div {
        card()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(24.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size(px(6.))
                                    .rounded_full()
                                    .bg(rgb(metric_color(metric))),
                            )
                            .child(heading(format!(
                                "{} · {}",
                                metric.title(),
                                if metric == Metric::Network {
                                    "收发速率"
                                } else {
                                    "使用率"
                                }
                            ))),
                    )
                    .child(caption(if metric == Metric::Network {
                        format!("纵轴 0–{}", metric.format_observation(trend.scale))
                    } else {
                        "纵轴 0–100%".into()
                    })),
            )
            .when(trend.points.is_empty(), |view| {
                view.child(
                    div().mb(px(20.)).child(heading("还没有历史记录")).child(
                        caption(if self.state.paused {
                            "继续监控以记录资源变化。"
                        } else {
                            "开始监控后，资源变化会显示在这里。"
                        })
                        .mt(px(6.)),
                    ),
                )
            })
            .child(trend_chart(trend, metric, 230.))
            .child(self.time_axis())
            .child(self.history_statistics(metric, trend))
    }

    fn history_statistics(&self, metric: Metric, trend: &Trend) -> Div {
        let has_samples = !trend.points.is_empty();
        let formatted = |value| {
            if has_samples {
                metric.format_observation(value)
            } else {
                "—".into()
            }
        };
        let statistics = [
            (
                "当前",
                if self.state.ready {
                    metric.value(&self.state.latest)
                } else {
                    "—".into()
                },
            ),
            ("平均", formatted(trend.average)),
            ("峰值", formatted(trend.peak)),
            ("记录数", format!("{} 个", trend.points.len())),
        ];
        div()
            .flex()
            .gap_4()
            .mt(px(24.))
            .pt(px(18.))
            .border_t_1()
            .border_color(rgb(BORDER))
            .children(statistics.into_iter().map(|(label, value)| {
                div().flex_1().child(caption(label)).child(
                    div()
                        .mt(px(8.))
                        .text_lg()
                        .font_weight(FontWeight::MEDIUM)
                        .child(value),
                )
            }))
    }
}
