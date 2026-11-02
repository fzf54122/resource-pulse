// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: sampler.rs
// @Software: resource-pulse

use crate::model::{Process, Snapshot};
use std::{
    collections::HashSet,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

pub(super) struct SystemSampler {
    system: System,
    disks: Disks,
    networks: Networks,
    started: Instant,
    previous: Instant,
    os: String,
    processes: Arc<[Process]>,
    last_inventory: Option<Instant>,
    last_capacity: Option<Instant>,
    capacity: (u64, u64),
}

impl SystemSampler {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        refresh_processes(&mut system);
        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            started: Instant::now(),
            previous: Instant::now(),
            os: System::name().unwrap_or_else(|| std::env::consts::OS.into()),
            processes: Arc::default(),
            last_inventory: None,
            last_capacity: None,
            capacity: (0, 0),
        }
    }

    pub fn reset_baselines(&mut self) {
        self.system.refresh_cpu_usage();
        refresh_processes(&mut self.system);
        self.networks.refresh();
        self.previous = Instant::now();
    }

    pub fn sample(&mut self, detail: bool) -> Snapshot {
        self.system.refresh_cpu_usage();
        if detail {
            refresh_processes(&mut self.system);
            self.processes = self
                .system
                .processes()
                .iter()
                .map(|(pid, process)| Process {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    cpu: process.cpu_usage(),
                    memory: process.memory(),
                })
                .collect();
        }
        let now = Instant::now();
        self.refresh_devices(now);
        let elapsed = now.duration_since(self.previous).as_secs_f64().max(0.001);
        self.previous = now;
        self.system.refresh_memory();
        let (disk_used, disk_total) = self.capacity;
        Snapshot {
            cpu: self.system.global_cpu_usage(),
            used_memory: self.system.used_memory(),
            total_memory: self.system.total_memory(),
            network_kib: (self
                .networks
                .values()
                .map(|data| data.received() as f64 + data.transmitted() as f64)
                .sum::<f64>()
                / 1024.
                / elapsed) as f32,
            disk_used,
            disk_total,
            cores: self.system.cpus().len(),
            uptime: System::uptime(),
            os: self.os.clone(),
            elapsed: self.started.elapsed().as_secs(),
            processes: self.processes.clone(),
        }
    }

    fn refresh_devices(&mut self, now: Instant) {
        // Device discovery and statfs are much more expensive than reading cached counters.
        if self
            .last_inventory
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(30))
        {
            self.networks.refresh_list();
            self.disks.refresh_list();
            self.last_inventory = Some(now);
            self.last_capacity = None;
        } else {
            self.networks.refresh();
        }
        if self
            .last_capacity
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(5))
        {
            self.disks.refresh();
            self.capacity = volume_capacity(self.disks.iter().map(|disk| {
                (
                    disk.mount_point(),
                    disk.total_space(),
                    disk.available_space(),
                )
            }));
            self.last_capacity = Some(now);
        }
    }
}

fn refresh_processes(system: &mut System) {
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        ProcessRefreshKind::new().with_cpu().with_memory(),
    );
}

fn volume_capacity<'a>(volumes: impl Iterator<Item = (&'a Path, u64, u64)>) -> (u64, u64) {
    let mut mount_points = HashSet::new();
    let mut used = 0_u64;
    let mut total = 0_u64;
    for (mount, capacity, available) in volumes {
        if mount_points.insert(mount) {
            total = total.saturating_add(capacity);
            used = used.saturating_add(capacity.saturating_sub(available));
        }
    }
    (used, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_updates_reuse_process_data_and_device_inventory() {
        let mut sampler = SystemSampler::new();
        let foreground = sampler.sample(true);
        let inventory_time = sampler.last_inventory;
        let capacity_time = sampler.last_capacity;
        let background = sampler.sample(false);
        assert!(Arc::ptr_eq(&foreground.processes, &background.processes));
        assert_eq!(sampler.last_inventory, inventory_time);
        assert_eq!(sampler.last_capacity, capacity_time);
        assert!(background.network_kib.is_finite());
    }

    #[test]
    fn capacity_counts_unique_mounts_and_clamps_invalid_free_space() {
        let volumes = [
            (Path::new("/"), 100, 30),
            (Path::new("/"), 100, 30),
            (Path::new("/data"), 200, 50),
            (Path::new("/empty"), 10, 20),
        ];
        assert_eq!(volume_capacity(volumes.into_iter()), (220, 310));
        assert_eq!(volume_capacity(std::iter::empty()), (0, 0));
    }
}
