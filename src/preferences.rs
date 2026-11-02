// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: preferences.rs
// @Software: resource-pulse

use crate::model::SamplingInterval;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub refresh_seconds: u64,
    pub close_to_tray: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            refresh_seconds: 1,
            close_to_tray: true,
        }
    }
}

impl Preferences {
    pub fn interval(self) -> io::Result<SamplingInterval> {
        match self.refresh_seconds {
            1 => Ok(SamplingInterval::One),
            2 => Ok(SamplingInterval::Two),
            5 => Ok(SamplingInterval::Five),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported refresh interval",
            )),
        }
    }
}

pub struct PreferenceStore {
    path: PathBuf,
}

impl PreferenceStore {
    pub fn for_current_user() -> io::Result<Self> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "user configuration directory unavailable",
                )
            })?;
        Ok(Self {
            path: base.join("resource-pulse/settings.json"),
        })
    }

    pub fn load(&self) -> io::Result<Preferences> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Preferences::default());
            }
            Err(error) => return Err(error),
        };
        let preferences: Preferences = serde_json::from_slice(&data)?;
        preferences.interval()?;
        Ok(preferences)
    }

    pub fn save(&self, preferences: Preferences) -> io::Result<()> {
        preferences.interval()?;
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "missing configuration parent")
        })?;
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(".settings-{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let result = (|| {
            file.write_all(&serde_json::to_vec_pretty(&preferences)?)?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)
        })();
        if result.is_err() {
            // This file was created exclusively by this save operation.
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn store() -> PreferenceStore {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        PreferenceStore {
            path: std::env::temp_dir().join(format!(
                "pulse-preferences-{}-{}/settings.json",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            )),
        }
    }

    #[test]
    fn preferences_round_trip_and_survive_reopening() {
        let store = store();
        assert_eq!(store.load().expect("defaults"), Preferences::default());
        let preferences = Preferences {
            refresh_seconds: 5,
            close_to_tray: false,
        };
        store.save(preferences).expect("save");
        assert_eq!(store.load().expect("reload"), preferences);
        fs::remove_file(&store.path).expect("remove test preference");
        fs::remove_dir(store.path.parent().expect("test directory"))
            .expect("remove empty test directory");
    }

    #[test]
    fn corrupt_and_unsupported_preferences_are_not_silently_overwritten() {
        let store = store();
        fs::create_dir_all(store.path.parent().expect("test directory")).expect("create directory");
        fs::write(&store.path, b"{invalid").expect("write fixture");
        assert!(store.load().is_err());
        assert_eq!(fs::read(&store.path).expect("read unchanged"), b"{invalid");
        assert!(
            store
                .save(Preferences {
                    refresh_seconds: 0,
                    close_to_tray: true
                })
                .is_err()
        );
        fs::remove_file(&store.path).expect("remove fixture");
        fs::remove_dir(store.path.parent().expect("test directory"))
            .expect("remove test directory");
    }
}
