// @Time    : 2026/10/30 18:04
// @Author  : fzf
// @FileName: lib.rs
// @Software: resource-pulse

//! Business-independent GPUI components. The caller owns state and click handlers.

mod chart;
mod components;
mod icon;
pub mod theme;

pub use chart::{ChartPoint, bar_chart, line_chart};
pub use components::{button, caption, card, heading, progress, section_header, status};
pub use gpui;
pub use gpui_platform::application;
pub use icon::{Icon, icon};
