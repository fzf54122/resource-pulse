// @Time    : 2026/10/30 18:04
// @Author  : fzf
// @FileName: tests.rs
// @Software: resource-pulse

use super::*;
use collector::Collector;
use gpui::{Context, Entity, Modifiers, TestAppContext, VisualTestContext};
use model::{HistoryRange, Metric, Page, Process, SamplingInterval, Snapshot};
use std::time::Duration;

fn fixture(cx: &mut Context<ResourcePulse>) -> ResourcePulse {
    let mut view = ResourcePulse::new(cx);
    view.state.ingest(Snapshot {
        elapsed: 1,
        cpu: 25.,
        used_memory: 4 * 1024_u64.pow(3),
        total_memory: 16 * 1024_u64.pow(3),
        cores: 8,
        processes: (1..=26)
            .map(|pid| Process {
                pid,
                name: format!("process-{pid}"),
                cpu: pid as f32,
                memory: u64::from(27 - pid) * 1024 * 1024,
            })
            .collect::<Vec<_>>()
            .into(),
        ..Snapshot::default()
    });
    view
}

fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, app| window.draw(app).clear(app));
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    redraw(cx);
    let bounds = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("missing clickable element {id}"));
    cx.simulate_click(bounds.center(), Modifiers::default());
    redraw(cx);
}

fn read<R>(
    view: &Entity<ResourcePulse>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&ResourcePulse) -> R,
) -> R {
    cx.update(|_, app| f(view.read(app)))
}

#[gpui::test]
fn sidebar_and_overview_links_navigate(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| fixture(cx));
    cx.simulate_resize(size(px(1220.), px(900.)));
    click(cx, "nav-processes");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::Processes);
    click(cx, "nav-history");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::History);
    click(cx, "nav-settings");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::Settings);
    click(cx, "nav-overview");
    click(cx, "cpu-card");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::History);
    click(cx, "nav-overview");
    click(cx, "view-processes");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::Processes);
}

#[gpui::test]
fn settings_and_pause_buttons_change_state(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| fixture(cx));
    click(cx, "pause");
    assert!(read(&view, cx, |v| v.state.paused));
    click(cx, "nav-settings");
    click(cx, "interval-5");
    assert_eq!(
        read(&view, cx, |v| v.state.interval),
        SamplingInterval::Five
    );
    click(cx, "settings-pause");
    assert!(!read(&view, cx, |v| v.state.paused));
    click(cx, "interval-2");
    assert_eq!(read(&view, cx, |v| v.state.interval), SamplingInterval::Two);
    click(cx, "interval-1");
    assert_eq!(read(&view, cx, |v| v.state.interval), SamplingInterval::One);
}

#[gpui::test]
fn process_sort_pagination_and_details_work(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| fixture(cx));
    cx.simulate_resize(size(px(1220.), px(1100.)));
    click(cx, "nav-processes");
    click(cx, "sort-memory");
    assert_eq!(read(&view, cx, |v| v.state.processes()[0].pid), 1);
    click(cx, "sort-memory");
    assert_eq!(read(&view, cx, |v| v.state.processes()[0].pid), 26);
    click(cx, "next-page");
    assert_eq!(read(&view, cx, |v| v.state.process_page), 1);
    click(cx, "previous-page");
    assert_eq!(read(&view, cx, |v| v.state.process_page), 0);
    click(cx, "process-26");
    assert_eq!(read(&view, cx, |v| v.state.selected_pid), Some(26));
    click(cx, "close-details");
    assert_eq!(read(&view, cx, |v| v.state.selected_pid), None);
}

#[gpui::test]
fn history_range_and_clear_buttons_work(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| fixture(cx));
    click(cx, "nav-history");
    click(cx, "range-30");
    assert_eq!(
        read(&view, cx, |v| v.state.range),
        HistoryRange::ThirtySeconds
    );
    click(cx, "range-300");
    assert_eq!(
        read(&view, cx, |v| v.state.range),
        HistoryRange::FiveMinutes
    );
    click(cx, "pause");
    click(cx, "clear-history");
    assert_eq!(read(&view, cx, |v| v.state.history.len()), 0);
}

#[gpui::test]
fn metric_cards_open_the_corresponding_history(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| fixture(cx));
    for metric in Metric::ALL {
        click(cx, "nav-overview");
        click(cx, metric.id());
        assert_eq!(read(&view, cx, |v| v.state.metric), metric);
        assert_eq!(read(&view, cx, |v| v.state.page), Page::History);
    }
    click(cx, "metric-cpu");
    assert_eq!(read(&view, cx, |v| v.state.metric), Metric::Cpu);
}

#[gpui::test]
fn keyboard_controls_sampling_and_process_details(cx: &mut TestAppContext) {
    cx.update(app::bind_keys);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let view = fixture(cx);
        view.focus.focus(window, cx);
        view
    });
    redraw(cx);
    cx.simulate_keystrokes("space");
    assert!(read(&view, cx, |v| v.state.paused));
    cx.simulate_keystrokes("space");
    assert!(!read(&view, cx, |v| v.state.paused));
    click(cx, "nav-processes");
    click(cx, "process-26");
    cx.simulate_keystrokes("escape");
    assert_eq!(read(&view, cx, |v| v.state.selected_pid), None);
}

#[gpui::test]
fn minimum_window_keeps_primary_controls_visible(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(|_, cx| fixture(cx));
    cx.simulate_resize(size(px(960.), px(640.)));
    for (nav, target) in [
        ("nav-overview", "disk-card"),
        ("nav-processes", "sort-cpu"),
        ("nav-history", "range-300"),
        ("nav-settings", "interval-5"),
    ] {
        click(cx, nav);
        let bounds = cx.debug_bounds(target).expect("primary control exists");
        assert!(bounds.left() >= px(184.));
        assert!(bounds.right() <= px(960.));
        assert!(bounds.top() >= px(0.));
        assert!(bounds.bottom() <= px(640.));
    }
}

#[gpui::test]
fn menubar_actions_share_navigation_and_monitoring_state(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| fixture(cx));
    click(cx, "menu-view");
    click(cx, "menu-processes");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::Processes);
    assert_eq!(read(&view, cx, |v| v.menu), None);
    click(cx, "menu-sampling");
    click(cx, "menu-toggle");
    assert!(read(&view, cx, |v| v.state.paused));
    click(cx, "menu-sampling");
    click(cx, "menu-interval-5");
    assert_eq!(
        read(&view, cx, |v| v.state.interval),
        SamplingInterval::Five
    );
    click(cx, "menu-app");
    click(cx, "menu-about");
    assert!(read(&view, cx, |v| v.about));
    click(cx, "close-about");
    assert!(!read(&view, cx, |v| v.about));
}

#[gpui::test]
fn settings_keep_close_behavior_and_shortcuts_consistent(cx: &mut TestAppContext) {
    cx.update(app::bind_keys);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let view = fixture(cx);
        view.focus.focus(window, cx);
        view
    });
    redraw(cx);
    cx.simulate_keystrokes("ctrl-,");
    assert_eq!(read(&view, cx, |v| v.state.page), Page::Settings);
    click(cx, "close-to-quit");
    assert!(!read(&view, cx, |v| v.preferences.close_to_tray));
    click(cx, "close-to-tray");
    assert!(read(&view, cx, |v| v.preferences.close_to_tray));
    click(cx, "settings-about");
    cx.simulate_keystrokes("escape");
    assert!(!read(&view, cx, |v| v.about));
}

#[test]
fn customer_facing_pages_do_not_expose_development_notes() {
    for source in [
        include_str!("views/settings.rs"),
        include_str!("views/shell.rs"),
        include_str!("views/history.rs"),
        include_str!("views/menubar.rs"),
    ] {
        for term in ["采样点", "GPUI", "后台线程", "本次会话", "仅在本次"] {
            assert!(
                !source.contains(term),
                "implementation note leaked into product UI: {term}"
            );
        }
    }
}

#[gpui::test]
fn failed_sampling_never_looks_live_or_toggles_pause(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        let mut view = fixture(cx);
        view.error = Some("测试采样中断".into());
        view
    });
    click(cx, "pause");
    assert!(!read(&view, cx, |v| v.state.paused));
    assert_eq!(read(&view, cx, |v| v.sampling_status()), "无法更新");
}

#[test]
fn system_collector_delivers_a_real_snapshot_and_stops() {
    let collector = Collector::start().expect("start system sampler");
    let samples = collector.samples.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(samples.recv_blocking());
    });
    let snapshot = receiver
        .recv_timeout(Duration::from_secs(15))
        .expect("receive real sample within deadline")
        .expect("sampler connected");
    assert!(snapshot.cpu.is_finite());
    assert!(snapshot.total_memory > 0);
    assert!(snapshot.cores > 0);
    assert!(snapshot.network_kib.is_finite());
    assert!(
        snapshot
            .processes
            .iter()
            .any(|p| p.pid == std::process::id())
    );
    drop(collector);
}
