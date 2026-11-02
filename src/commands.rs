// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: commands.rs
// @Software: resource-pulse

use crate::model::{Page, SamplingInterval};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppCommand {
    Show,
    Navigate(Page),
    ToggleSampling,
    Interval(SamplingInterval),
    ClearHistory,
    CopySummary,
    Minimize,
    About,
    Quit,
    TrayAvailable(bool),
    GlobalMenuAvailable(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuGroup {
    Application,
    View,
    Sampling,
    Window,
}

impl MenuGroup {
    pub const ALL: [Self; 4] = [Self::Application, Self::View, Self::Sampling, Self::Window];

    pub fn id(self) -> &'static str {
        match self {
            Self::Application => "menu-app",
            Self::View => "menu-view",
            Self::Sampling => "menu-sampling",
            Self::Window => "menu-window",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Application => "Pulse",
            Self::View => "查看",
            Self::Sampling => "监控",
            Self::Window => "窗口",
        }
    }

    pub fn offset(self) -> f32 {
        match self {
            Self::Application => 12.,
            Self::View => 80.,
            Self::Sampling => 136.,
            Self::Window => 192.,
        }
    }
}
