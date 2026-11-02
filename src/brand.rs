// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: brand.rs
// @Software: resource-pulse

use pulse_ui::gpui::{Image, ImageFormat};
use std::sync::{Arc, LazyLock};

pub const APP_ID: &str = "io.github.fzf.Pulse";
const ICON: &[u8] = include_bytes!("../assets/pulse-256.png");

pub fn mascot() -> Arc<Image> {
    static IMAGE: LazyLock<Arc<Image>> =
        LazyLock::new(|| Arc::new(Image::from_bytes(ImageFormat::Png, ICON.to_vec())));
    IMAGE.clone()
}

pub fn window_icon() -> Result<Arc<image::RgbaImage>, image::ImageError> {
    image::load_from_memory_with_format(ICON, image::ImageFormat::Png)
        .map(|image| Arc::new(image.into_rgba8()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_mascot_is_valid_and_has_alpha() {
        let image = super::window_icon().expect("decode embedded mascot");
        assert_eq!(image.dimensions(), (256, 256));
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        assert!(image.get_pixel(128, 128).0[3] > 0);
    }
}
