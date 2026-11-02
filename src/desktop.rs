// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: desktop.rs
// @Software: resource-pulse

#[cfg(target_os = "linux")]
mod global_menu;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod tray_icon;
#[cfg(target_os = "linux")]
pub use global_menu::GlobalMenuIntegration;
#[cfg(target_os = "linux")]
pub use linux::{Instance, TrayIntegration};

#[cfg(not(target_os = "linux"))]
mod fallback {
    use crate::commands::AppCommand;
    use crate::menu::MenuState;
    use async_channel::Sender;
    use pulse_ui::gpui::Window;

    pub struct Instance;
    impl Instance {
        pub fn acquire(_: Sender<AppCommand>) -> Result<Option<Self>, String> {
            Ok(Some(Self))
        }
    }

    #[derive(Clone)]
    pub struct TrayIntegration;
    impl TrayIntegration {
        pub async fn start(_: Sender<AppCommand>) -> Result<Self, String> {
            Err("当前平台尚未接入系统托盘。".into())
        }
        pub async fn set_paused(&self, _: bool) {}
    }

    #[derive(Clone)]
    pub struct GlobalMenuIntegration;
    impl GlobalMenuIntegration {
        pub fn window_id(_: &Window) -> Option<u32> {
            None
        }
        pub async fn start(
            _: u32,
            _: Sender<AppCommand>,
            _: MenuState,
        ) -> Result<(Self, bool), String> {
            Err("当前窗口系统不支持全局菜单。".into())
        }
        pub fn set_state(&self, _: MenuState) -> Result<Option<u32>, String> {
            Ok(None)
        }
        pub async fn notify(&self, _: u32) -> Result<(), String> {
            Ok(())
        }
        pub async fn watch(self, _: Sender<AppCommand>) -> Result<(), String> {
            Ok(())
        }
    }
}
#[cfg(not(target_os = "linux"))]
pub use fallback::{GlobalMenuIntegration, Instance, TrayIntegration};
