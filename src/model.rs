// @Time    : 2026/10/30 18:04
// @Author  : fzf
// @FileName: model.rs
// @Software: resource-pulse

use std::{collections::VecDeque, sync::Arc};

mod format;
mod metric;
pub use format::{bytes, duration, rate, ratio};
pub use metric::{Metric, Trend};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Overview,
    Processes,
    History,
    Settings,
}

impl Page {
    pub const ALL: [Self; 4] = [
        Self::Overview,
        Self::Processes,
        Self::History,
        Self::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "概览",
            Self::Processes => "进程",
            Self::History => "资源历史",
            Self::Settings => "设置",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Overview => "当前设备的资源与运行状态",
            Self::Processes => "按资源占用检查本机进程",
            Self::History => "查看近期资源使用情况",
            Self::Settings => "按你的习惯使用 Pulse",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SamplingInterval {
    #[default]
    One,
    Two,
    Five,
}

impl SamplingInterval {
    pub const ALL: [Self; 3] = [Self::One, Self::Two, Self::Five];

    pub fn seconds(self) -> u64 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Five => 5,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::One => "interval-1",
            Self::Two => "interval-2",
            Self::Five => "interval-5",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HistoryRange {
    ThirtySeconds,
    #[default]
    Minute,
    FiveMinutes,
}

impl HistoryRange {
    pub const ALL: [Self; 3] = [Self::ThirtySeconds, Self::Minute, Self::FiveMinutes];

    pub fn seconds(self) -> u64 {
        match self {
            Self::ThirtySeconds => 30,
            Self::Minute => 60,
            Self::FiveMinutes => 300,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::ThirtySeconds => "range-30",
            Self::Minute => "range-60",
            Self::FiveMinutes => "range-300",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Cpu,
    Memory,
    Name,
}

#[derive(Clone, Debug)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory: u64,
}

#[derive(Default)]
pub struct Snapshot {
    pub cpu: f32,
    pub used_memory: u64,
    pub total_memory: u64,
    pub network_kib: f32,
    pub disk_used: u64,
    pub disk_total: u64,
    pub cores: usize,
    pub uptime: u64,
    pub os: String,
    pub processes: Arc<[Process]>,
    pub elapsed: u64,
}

#[derive(Clone, Copy)]
pub struct HistoryPoint {
    pub elapsed: u64,
    pub cpu: Option<f32>,
    pub memory: Option<f32>,
    pub network: Option<f32>,
    pub disk: Option<f32>,
    pub interval: u64,
}

pub struct Dashboard {
    pub page: Page,
    pub paused: bool,
    pub interval: SamplingInterval,
    pub range: HistoryRange,
    pub metric: Metric,
    pub sort: Sort,
    pub descending: bool,
    pub selected_pid: Option<u32>,
    pub process_page: usize,
    pub latest: Snapshot,
    pub history: VecDeque<HistoryPoint>,
    pub ready: bool,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self {
            page: Page::Overview,
            paused: false,
            interval: SamplingInterval::default(),
            range: HistoryRange::default(),
            metric: Metric::default(),
            sort: Sort::Cpu,
            descending: true,
            selected_pid: None,
            process_page: 0,
            latest: Snapshot::default(),
            history: VecDeque::new(),
            ready: false,
        }
    }
}

pub const PAGE_SIZE: usize = 12;
pub const HISTORY_CAPACITY: usize = 300;

impl Dashboard {
    pub fn ingest(&mut self, snapshot: Snapshot) {
        if self.paused {
            return;
        }
        self.history.push_back(HistoryPoint {
            elapsed: snapshot.elapsed,
            cpu: Metric::Cpu.reading(&snapshot),
            memory: Metric::Memory.reading(&snapshot),
            network: Metric::Network.reading(&snapshot),
            disk: Metric::Disk.reading(&snapshot),
            interval: self.interval.seconds(),
        });
        while self.history.len() > HISTORY_CAPACITY {
            self.history.pop_front();
        }
        self.latest = snapshot;
        self.ready = true;
        self.process_page = self.process_page.min(self.page_count().saturating_sub(1));
    }

    pub fn set_sort(&mut self, sort: Sort) {
        self.descending = if self.sort == sort {
            !self.descending
        } else {
            sort != Sort::Name
        };
        self.sort = sort;
        self.process_page = 0;
    }

    pub fn navigate(&mut self, page: Page) {
        self.page = page;
    }

    pub fn inspect_metric(&mut self, metric: Metric) {
        self.metric = metric;
        self.navigate(Page::History);
    }

    pub fn select_process(&mut self, pid: u32) {
        self.selected_pid = Some(pid);
        self.navigate(Page::Processes);
    }

    pub fn previous_page(&mut self) {
        self.process_page = self.process_page.saturating_sub(1);
    }

    pub fn next_page(&mut self) {
        self.process_page = (self.process_page + 1).min(self.page_count() - 1);
    }

    pub fn selected_process(&self) -> Option<&Process> {
        self.selected_pid
            .and_then(|pid| self.latest.processes.iter().find(|p| p.pid == pid))
    }

    pub fn process_bounds(&self) -> (usize, usize) {
        let total = self.latest.processes.len();
        let start = (self.process_page * PAGE_SIZE).min(total);
        (start, (start + PAGE_SIZE).min(total))
    }

    pub fn processes(&self) -> Vec<&Process> {
        let mut rows: Vec<_> = self.latest.processes.iter().collect();
        rows.sort_by(|a, b| {
            let order = match self.sort {
                Sort::Cpu => a.cpu.total_cmp(&b.cpu),
                Sort::Memory => a.memory.cmp(&b.memory),
                Sort::Name => a.name.cmp(&b.name),
            };
            let order = if self.descending {
                order.reverse()
            } else {
                order
            };
            order.then(a.pid.cmp(&b.pid))
        });
        rows
    }

    pub fn page_count(&self) -> usize {
        self.latest.processes.len().div_ceil(PAGE_SIZE).max(1)
    }

    pub fn visible_history(&self) -> Vec<HistoryPoint> {
        let cutoff = self.latest.elapsed.saturating_sub(self.range.seconds());
        self.history
            .iter()
            .filter(|point| point.elapsed >= cutoff)
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(elapsed: u64) -> Snapshot {
        Snapshot {
            elapsed,
            processes: vec![
                Process {
                    pid: 3,
                    name: "beta".into(),
                    cpu: 10.,
                    memory: 300,
                },
                Process {
                    pid: 2,
                    name: "alpha".into(),
                    cpu: 20.,
                    memory: 100,
                },
                Process {
                    pid: 1,
                    name: "gamma".into(),
                    cpu: 20.,
                    memory: 200,
                },
            ]
            .into(),
            ..Snapshot::default()
        }
    }

    #[test]
    fn sorting_toggles_and_ties_are_stable() {
        let mut state = Dashboard::default();
        state.ingest(snapshot(1));
        assert_eq!(
            state.processes().iter().map(|p| p.pid).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        state.set_sort(Sort::Memory);
        assert_eq!(state.processes()[0].pid, 3);
        state.set_sort(Sort::Memory);
        assert_eq!(state.processes()[0].pid, 2);
        state.set_sort(Sort::Name);
        assert_eq!(state.processes()[0].name, "alpha");
    }

    #[test]
    fn pause_freezes_snapshot_and_history_then_resumes() {
        let mut state = Dashboard::default();
        state.ingest(snapshot(1));
        state.paused = true;
        state.ingest(snapshot(2));
        assert_eq!(state.latest.elapsed, 1);
        assert_eq!(state.history.len(), 1);
        state.paused = false;
        state.ingest(snapshot(3));
        assert_eq!(state.latest.elapsed, 3);
    }

    #[test]
    fn history_is_bounded_and_filtered_by_time() {
        let mut state = Dashboard::default();
        for elapsed in 0..1000 {
            state.ingest(snapshot(elapsed));
        }
        assert_eq!(state.history.len(), 300);
        state.range = HistoryRange::ThirtySeconds;
        assert_eq!(state.visible_history().len(), 31);
        state.history.clear();
        assert!(state.visible_history().is_empty());
    }

    #[test]
    fn shrinking_process_list_clamps_pagination() {
        let mut state = Dashboard {
            process_page: 99,
            ..Dashboard::default()
        };
        state.ingest(snapshot(1));
        assert_eq!(state.process_page, 0);
        state.ingest(Snapshot::default());
        assert_eq!(state.page_count(), 1);
        assert!(state.processes().is_empty());
    }

    #[test]
    fn pagination_stays_in_bounds_and_reports_visible_rows() {
        let mut state = Dashboard::default();
        state.ingest(Snapshot {
            processes: (1..=26)
                .map(|pid| Process {
                    pid,
                    name: format!("process-{pid}"),
                    cpu: 0.,
                    memory: 0,
                })
                .collect::<Vec<_>>()
                .into(),
            ..Snapshot::default()
        });
        state.previous_page();
        assert_eq!(state.process_bounds(), (0, 12));
        state.next_page();
        assert_eq!(state.process_bounds(), (12, 24));
        state.next_page();
        state.next_page();
        assert_eq!(state.process_bounds(), (24, 26));
        state.set_sort(Sort::Name);
        assert_eq!(state.process_page, 0);
    }

    #[test]
    fn process_selection_survives_exit_without_reusing_stale_details() {
        let mut state = Dashboard::default();
        state.ingest(snapshot(1));
        state.select_process(2);
        assert_eq!(state.page, Page::Processes);
        assert_eq!(state.selected_process().map(|p| p.pid), Some(2));
        state.ingest(Snapshot::default());
        assert!(state.selected_process().is_none());
        assert_eq!(state.selected_pid, Some(2));
    }

    #[test]
    fn ratios_handle_zero_and_invalid_totals() {
        assert_eq!(ratio(12, 0), 0.);
        assert_eq!(ratio(20, 10), 1.);
        assert_eq!(ratio(5, 10), 0.5);
    }
}
