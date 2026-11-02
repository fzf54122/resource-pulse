// @Time    : 2026/11/2 08:35
// @Author  : fzf
// @FileName: tray_icon.rs
// @Software: resource-pulse

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

const SVG: &[u8] = include_bytes!("../../assets/pulse-tray.svg");
const SOURCES: &[(i32, &[u8])] = &[
    (16, include_bytes!("../../assets/tray/pulse-16.png")),
    (22, include_bytes!("../../assets/tray/pulse-22.png")),
    (24, include_bytes!("../../assets/tray/pulse-24.png")),
    (32, include_bytes!("../../assets/tray/pulse-32.png")),
    (48, include_bytes!("../../assets/tray/pulse-48.png")),
    (64, include_bytes!("../../assets/tray/pulse-64.png")),
    (96, include_bytes!("../../assets/tray/pulse-96.png")),
];

pub(super) struct TrayIcon {
    pub name: String,
    pub pixmaps: Vec<ksni::Icon>,
}

impl TrayIcon {
    pub fn load() -> Result<Self, image::ImageError> {
        let pixmaps = pixmaps()?;
        let name = match cache_directory().and_then(|directory| materialize(&directory)) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(error) => {
                // An empty name makes the host use IconPixmap instead of a missing theme icon.
                eprintln!("Pulse 托盘矢量图标不可用，将使用内嵌图标：{error}");
                String::new()
            }
        };
        Ok(Self { name, pixmaps })
    }
}

fn cache_directory() -> io::Result<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .filter(|path| path.is_absolute())
        .map(|path| path.join("resource-pulse/icons"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "user cache directory unavailable"))
}

fn materialize(directory: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(directory)?;
    let target = directory.join(concat!("pulse-tray-", env!("CARGO_PKG_VERSION"), ".svg"));
    if target.try_exists()? {
        if fs::read(&target)? == SVG {
            return Ok(target);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cached tray icon differs from embedded asset",
        ));
    }
    let temporary = directory.join(format!(".pulse-tray-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(SVG)?;
        file.sync_all()?;
        fs::rename(&temporary, &target)?;
        Ok(target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn pixmaps() -> Result<Vec<ksni::Icon>, image::ImageError> {
    SOURCES
        .iter()
        .map(|(size, bytes)| {
            let image =
                image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgba8();
            Ok(ksni::Icon {
                width: *size,
                height: *size,
                data: argb(&image),
            })
        })
        .collect()
}

fn argb(image: &image::RgbaImage) -> Vec<u8> {
    image
        .pixels()
        .flat_map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            [alpha, red, green, blue]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixmaps_use_actual_native_sizes_with_network_argb_and_transparency() {
        let icons = pixmaps().expect("embedded tray icons");
        assert_eq!(
            icons.iter().map(|icon| icon.width).collect::<Vec<_>>(),
            [16, 22, 24, 32, 48, 64, 96]
        );
        for icon in icons {
            assert_eq!(icon.width, icon.height);
            assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
            assert!(
                icon.data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[0] == 0)
            );
            assert!(
                icon.data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[0] == 255)
            );
        }
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([12, 34, 56, 78]));
        assert_eq!(argb(&image), [78, 12, 34, 56]);
    }

    #[test]
    fn vector_icon_is_materialized_without_overwriting_existing_data() {
        let directory =
            std::env::temp_dir().join(format!("pulse-tray-icon-{}", std::process::id()));
        fs::create_dir(&directory).expect("unique test directory");
        let target = materialize(&directory).expect("materialize");
        assert!(target.is_absolute());
        assert_eq!(fs::read(&target).expect("read svg"), SVG);
        assert_eq!(materialize(&directory).expect("reuse"), target);
        fs::write(&target, "test cache conflict").expect("fixture");
        assert!(materialize(&directory).is_err());
        assert_eq!(
            fs::read_to_string(&target).expect("unchanged"),
            "test cache conflict"
        );
        fs::remove_file(target).expect("remove fixture");
        fs::remove_dir(directory).expect("remove empty test directory");
    }
}
