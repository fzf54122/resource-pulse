// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: metric.rs
// @Software: resource-pulse

use super::{Dashboard, HistoryPoint, Snapshot, bytes, rate, ratio};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metric {
    #[default]
    Cpu,
    Memory,
    Network,
    Disk,
}

impl Metric {
    pub const ALL: [Self; 4] = [Self::Cpu, Self::Memory, Self::Network, Self::Disk];

    pub fn title(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "内存",
            Self::Network => "网络",
            Self::Disk => "存储空间",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Cpu => "cpu-card",
            Self::Memory => "memory-card",
            Self::Network => "network-card",
            Self::Disk => "disk-card",
        }
    }

    pub fn tab_id(self) -> &'static str {
        match self {
            Self::Cpu => "metric-cpu",
            Self::Memory => "metric-memory",
            Self::Network => "metric-network",
            Self::Disk => "metric-disk",
        }
    }

    pub fn value(self, sample: &Snapshot) -> String {
        self.reading(sample)
            .map(|value| self.format_observation(value))
            .unwrap_or_else(|| "—".into())
    }

    pub(super) fn reading(self, sample: &Snapshot) -> Option<f32> {
        let value = match self {
            Self::Cpu => sample.cpu / 100.,
            Self::Memory if sample.total_memory > 0 => {
                ratio(sample.used_memory, sample.total_memory)
            }
            Self::Network => sample.network_kib,
            Self::Disk if sample.disk_total > 0 => ratio(sample.disk_used, sample.disk_total),
            Self::Memory | Self::Disk => return None,
        };
        value.is_finite().then(|| {
            if self == Self::Network {
                value.max(0.)
            } else {
                value.clamp(0., 1.)
            }
        })
    }

    pub fn detail(self, sample: &Snapshot) -> String {
        match self {
            Self::Cpu => format!("{} 个逻辑核心", sample.cores),
            Self::Memory if sample.total_memory == 0 => "内存信息不可用".into(),
            Self::Memory => format!(
                "{} / {}",
                bytes(sample.used_memory),
                bytes(sample.total_memory)
            ),
            Self::Network => "发送与接收".into(),
            Self::Disk if sample.disk_total == 0 => "容量信息不可用".into(),
            Self::Disk => format!("{} / {}", bytes(sample.disk_used), bytes(sample.disk_total)),
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            Self::Cpu => "所有 CPU 核心的整体使用率。",
            Self::Memory => "已用物理内存占总物理内存的比例。",
            Self::Network => "所有网络接口的收发总量。",
            Self::Disk => "显示已挂载卷的容量使用情况，不代表磁盘读写速度。",
        }
    }

    fn observation(self, point: &HistoryPoint) -> Option<f32> {
        match self {
            Self::Cpu => point.cpu,
            Self::Memory => point.memory,
            Self::Network => point.network,
            Self::Disk => point.disk,
        }
    }

    pub fn format_observation(self, value: f32) -> String {
        if self == Self::Network {
            rate(value)
        } else {
            format!("{:.1}%", value * 100.)
        }
    }
}

pub struct TrendPoint {
    pub x: f32,
    pub y: f32,
    pub break_before: bool,
}

pub struct Trend {
    pub points: Vec<TrendPoint>,
    pub peak: f32,
    pub average: f32,
    pub scale: f32,
}

impl Dashboard {
    pub fn trend(&self, metric: Metric) -> Trend {
        let history = self.visible_history();
        let values: Vec<_> = history.iter().map(|p| metric.observation(p)).collect();
        let count = values.iter().flatten().count();
        let peak = values.iter().flatten().copied().fold(0., f32::max);
        let average = if count == 0 {
            0.
        } else {
            (values.iter().flatten().map(|v| f64::from(*v)).sum::<f64>() / count as f64) as f32
        };
        let scale = if metric == Metric::Network {
            peak.max(1.)
        } else {
            1.
        };
        let range = self.range.seconds() as f64;
        let points = history
            .iter()
            .zip(&values)
            .enumerate()
            .filter_map(|(index, (point, value))| {
                let value = (*value)?;
                // Use the requested time domain even before a full window has accumulated.
                let age = self.latest.elapsed.saturating_sub(point.elapsed) as f64;
                Some(TrendPoint {
                    x: (1. - age / range).clamp(0., 1.) as f32,
                    y: (value / scale).clamp(0., 1.),
                    break_before: index == 0
                        || values[index - 1].is_none()
                        || point.elapsed.saturating_sub(history[index - 1].elapsed)
                            > point.interval + 1,
                })
            })
            .collect();
        Trend {
            points,
            peak,
            average,
            scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::HistoryRange;

    #[test]
    fn disk_trend_contains_real_capacity_samples() {
        let mut state = Dashboard::default();
        for (elapsed, disk_used) in [(1, 25), (2, 75)] {
            state.ingest(Snapshot {
                elapsed,
                disk_used,
                disk_total: 100,
                ..Snapshot::default()
            });
        }
        let trend = state.trend(Metric::Disk);
        assert_eq!(trend.points.len(), 2);
        assert_eq!(trend.points[0].y, 0.25);
        assert_eq!(trend.points[1].y, 0.75);
        assert_eq!(trend.average, 0.5);
    }

    #[test]
    fn time_axis_preserves_pause_gaps_and_short_sessions() {
        let mut state = Dashboard {
            range: HistoryRange::ThirtySeconds,
            ..Dashboard::default()
        };
        for elapsed in [1, 2, 20, 21] {
            state.ingest(Snapshot {
                elapsed,
                ..Snapshot::default()
            });
        }
        let trend = state.trend(Metric::Cpu);
        assert!((trend.points[0].x - 1. / 3.).abs() < 0.001);
        assert!(!trend.points[1].break_before);
        assert!(trend.points[2].break_before);
        assert!(!trend.points[3].break_before);
        assert_eq!(trend.points[3].x, 1.);
    }

    #[test]
    fn network_uses_peak_scale_and_handles_nonfinite_samples() {
        let mut state = Dashboard::default();
        for (elapsed, network_kib) in [(1, 100.), (2, 200.), (3, f32::NAN), (4, 50.)] {
            state.ingest(Snapshot {
                elapsed,
                network_kib,
                ..Snapshot::default()
            });
        }
        let trend = state.trend(Metric::Network);
        assert_eq!(trend.scale, 200.);
        assert_eq!(trend.points[0].y, 0.5);
        assert_eq!(trend.points.len(), 3);
        assert_eq!(trend.points[2].y, 0.25);
        assert!(trend.points[2].break_before);
        state.history.clear();
        assert!(state.trend(Metric::Cpu).points.is_empty());
    }

    #[test]
    fn unavailable_capacity_is_not_reported_as_zero_usage() {
        let sample = Snapshot::default();
        for metric in [Metric::Memory, Metric::Disk] {
            assert_eq!(metric.value(&sample), "—");
            let mut state = Dashboard::default();
            state.ingest(Snapshot::default());
            assert!(state.trend(metric).points.is_empty());
        }
        assert_eq!(
            Metric::Cpu.value(&Snapshot {
                cpu: f32::NAN,
                ..Snapshot::default()
            }),
            "—"
        );
    }
}
