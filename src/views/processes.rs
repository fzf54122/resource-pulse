// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: processes.rs
// @Software: resource-pulse

use super::*;
use crate::model::Process;
use pulse_ui::gpui::Stateful;

const PID_WIDTH: f32 = 68.;
const MEMORY_WIDTH: f32 = 116.;
const CPU_WIDTH: f32 = 126.;

impl ResourcePulse {
    pub(super) fn processes(&self, cx: &mut Context<Self>) -> Div {
        div()
            .child(section_header(
                format!("{} 个进程", self.state.latest.processes.len()),
                "点击表头排序 · 点击进程查看详情",
            ))
            .child(self.process_table(false, cx))
            .child(self.process_pagination(cx))
            .when_some(self.state.selected_pid, |view, pid| {
                view.child(self.process_details(pid, cx))
            })
            .child(caption("CPU 使用率按单个核心计算，多核进程可能超过 100%。").mt(px(18.)))
    }

    pub(super) fn process_table(&self, preview: bool, cx: &mut Context<Self>) -> Div {
        let rows = self.state.processes();
        let start = if preview {
            0
        } else {
            self.state.process_bounds().0
        };
        let limit = if preview { 4 } else { PAGE_SIZE };
        card()
            .p_0()
            .overflow_hidden()
            .child(self.process_header(cx))
            .when(rows.is_empty(), |view| view.child(self.empty_processes()))
            .children(
                rows.into_iter()
                    .skip(start)
                    .take(limit)
                    .map(|process| self.process_row(process, cx)),
            )
    }

    fn process_header(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap_2()
            .px(px(14.))
            .py(px(6.))
            .bg(rgb(BACKGROUND))
            .child(self.sort_header(Sort::Name, cx).flex_1().justify_start())
            .child(caption("PID").w(px(PID_WIDTH)).text_right())
            .child(
                self.sort_header(Sort::Memory, cx)
                    .w(px(MEMORY_WIDTH))
                    .justify_end(),
            )
            .child(
                self.sort_header(Sort::Cpu, cx)
                    .w(px(CPU_WIDTH))
                    .justify_end(),
            )
    }

    fn sort_header(&self, sort: Sort, cx: &mut Context<Self>) -> Stateful<Div> {
        let (id, label) = match sort {
            Sort::Name => ("sort-name", "进程名称"),
            Sort::Memory => ("sort-memory", "内存"),
            Sort::Cpu => ("sort-cpu", "CPU"),
        };
        let arrow = if self.state.sort != sort {
            ""
        } else if self.state.descending {
            " ↓"
        } else {
            " ↑"
        };
        self.control(
            id,
            format!("{label}{arrow}"),
            false,
            move |this, _| this.state.set_sort(sort),
            cx,
        )
        .border_0()
        .bg(rgb(BACKGROUND))
        .px_0()
    }

    fn empty_processes(&self) -> Div {
        div()
            .py(px(40.))
            .text_center()
            .child(heading(if self.state.ready {
                "没有可见进程"
            } else {
                "正在读取进程"
            }))
            .child(
                caption(if self.state.ready {
                    "系统权限可能限制了进程信息的读取。"
                } else {
                    "进程信息将在读取完成后显示。"
                })
                .mt(px(8.)),
            )
    }

    fn process_row(&self, process: &Process, cx: &mut Context<Self>) -> Stateful<Div> {
        let pid = process.pid;
        div()
            .id(("process", u64::from(pid)))
            .debug_selector(move || format!("process-{pid}"))
            .flex()
            .items_center()
            .gap_2()
            .h(px(44.))
            .px(px(14.))
            .border_t_1()
            .border_color(rgb(BORDER))
            .cursor_pointer()
            .when(self.state.selected_pid == Some(pid), |view| {
                view.bg(rgb(TINT))
            })
            .hover(|view| view.bg(rgb(theme::HOVER)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.state.select_process(pid);
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .text_ellipsis()
                    .child(process.name.clone()),
            )
            .child(caption(pid.to_string()).w(px(PID_WIDTH)).text_right())
            .child(
                div()
                    .w(px(MEMORY_WIDTH))
                    .text_right()
                    .text_xs()
                    .child(bytes(process.memory)),
            )
            .child(
                div()
                    .w(px(CPU_WIDTH))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_3()
                    .child(div().w(px(40.)).child(progress(
                        process.cpu / (self.state.latest.cores.max(1) as f32 * 100.),
                        ACCENT,
                    )))
                    .child(
                        div()
                            .w(px(62.))
                            .text_right()
                            .text_xs()
                            .child(format!("{:.1}%", process.cpu)),
                    ),
            )
    }

    fn process_pagination(&self, cx: &mut Context<Self>) -> Div {
        let (start, end) = self.state.process_bounds();
        let total = self.state.latest.processes.len();
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .mt(px(14.))
            .child(caption(if total == 0 {
                "0 个进程".into()
            } else {
                format!("显示 {}–{end}，共 {total} 个", start + 1)
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        self.control(
                            "previous-page",
                            "上一页",
                            false,
                            |this, _| this.state.previous_page(),
                            cx,
                        )
                        .when(self.state.process_page == 0, |view| {
                            view.opacity(0.35).cursor_default()
                        }),
                    )
                    .child(caption(format!(
                        "{} / {}",
                        self.state.process_page + 1,
                        self.state.page_count()
                    )))
                    .child(
                        self.control(
                            "next-page",
                            "下一页",
                            false,
                            |this, _| this.state.next_page(),
                            cx,
                        )
                        .when(
                            self.state.process_page + 1 >= self.state.page_count(),
                            |view| view.opacity(0.35).cursor_default(),
                        ),
                    ),
            )
    }

    fn process_details(&self, pid: u32, cx: &mut Context<Self>) -> Div {
        card()
            .mt(px(20.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(heading("进程详情"))
                    .child(self.control(
                        "close-details",
                        "关闭 · Esc",
                        false,
                        |this, _| this.state.selected_pid = None,
                        cx,
                    )),
            )
            .child(key_value("PID", pid.to_string()))
            .when_some(self.state.selected_process(), |view, process| {
                view.child(key_value("名称", process.name.clone()))
                    .child(key_value("CPU", format!("{:.1}%", process.cpu)))
                    .child(key_value("内存", bytes(process.memory)))
            })
            .when(self.state.selected_process().is_none(), |view| {
                view.child(caption("该进程已退出或当前不可访问。").mt(px(8.)))
            })
    }
}
