// @Time    : 2026/10/30 18:04
// @Author  : fzf
// @FileName: collector.rs
// @Software: resource-pulse

mod sampler;

use crate::model::{SamplingInterval, Snapshot};
use async_channel::{Receiver, TrySendError};
use sampler::SystemSampler;
use std::{
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

enum Control {
    Pause(bool),
    Interval(SamplingInterval),
    Detail(bool),
    Stop,
}

/// The worker sleeps on commands while paused; samples wake GPUI without a polling timer.
pub struct Collector {
    pub samples: Receiver<Snapshot>,
    control: Sender<Control>,
    worker: Option<JoinHandle<()>>,
}

impl Collector {
    pub fn start() -> std::io::Result<Self> {
        let (sender, samples) = async_channel::bounded(1);
        let (control, commands) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("pulse-sampler".into())
            .spawn(move || {
                let mut sampler = SystemSampler::new();
                let mut interval = SamplingInterval::default();
                let mut paused = false;
                let mut detail = true;
                loop {
                    let command = if paused {
                        commands
                            .recv()
                            .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
                    } else {
                        commands.recv_timeout(Duration::from_secs(interval.seconds()))
                    };
                    match command {
                        Ok(Control::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Ok(Control::Pause(value)) => {
                            if paused && !value {
                                sampler.reset_baselines();
                            }
                            paused = value;
                        }
                        Ok(Control::Interval(value)) => interval = value,
                        Ok(Control::Detail(value)) => {
                            if !detail && value && !paused {
                                sampler.reset_baselines();
                            }
                            detail = value;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            match sender.try_send(sampler.sample(detail)) {
                                Ok(()) | Err(TrySendError::Full(_)) => {}
                                Err(TrySendError::Closed(_)) => break,
                            }
                        }
                    }
                }
            })?;
        Ok(Self {
            samples,
            control,
            worker: Some(worker),
        })
    }

    pub fn set_paused(&self, paused: bool) {
        self.send(Control::Pause(paused));
    }

    pub fn set_interval(&self, interval: SamplingInterval) {
        self.send(Control::Interval(interval));
    }

    pub fn set_detail(&self, visible: bool) {
        self.send(Control::Detail(visible));
    }

    fn send(&self, command: Control) {
        // A disconnected receiver also closes `samples`, which the application reports.
        let _ = self.control.send(command);
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        self.send(Control::Stop);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            eprintln!("Pulse sampler stopped unexpectedly");
        }
    }
}
