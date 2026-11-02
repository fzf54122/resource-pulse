// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: linux.rs
// @Software: resource-pulse

use super::tray_icon::TrayIcon;
use crate::{brand::APP_ID, commands::AppCommand, model::Page};
use async_channel::Sender;
use ksni::{MenuItem, Tray, TrayMethods, menu::StandardItem};

const OBJECT_PATH: &str = "/io/github/fzf/Pulse";

pub struct Instance {
    _connection: zbus::blocking::Connection,
}

struct Activation(Sender<AppCommand>);

#[zbus::interface(name = "io.github.fzf.Pulse")]
impl Activation {
    fn show(&self) -> zbus::fdo::Result<()> {
        self.0
            .try_send(AppCommand::Show)
            .map_err(|_| zbus::fdo::Error::Failed("应用当前无法处理激活请求".into()))
    }
}

impl Instance {
    /// A second launch activates the existing window instead of allocating another GPU context.
    pub fn acquire(commands: Sender<AppCommand>) -> Result<Option<Self>, String> {
        let connection = zbus::blocking::Connection::session().map_err(|e| e.to_string())?;
        connection
            .object_server()
            .at(OBJECT_PATH, Activation(commands))
            .map_err(|e| e.to_string())?;
        match connection
            .request_name_with_flags(APP_ID, zbus::fdo::RequestNameFlags::DoNotQueue.into())
        {
            Ok(
                zbus::fdo::RequestNameReply::PrimaryOwner
                | zbus::fdo::RequestNameReply::AlreadyOwner,
            ) => Ok(Some(Self {
                _connection: connection,
            })),
            Ok(_) | Err(zbus::Error::NameTaken) => {
                let proxy = zbus::blocking::Proxy::new(&connection, APP_ID, OBJECT_PATH, APP_ID)
                    .map_err(|e| e.to_string())?;
                proxy
                    .call::<_, _, ()>("Show", &())
                    .map_err(|e| e.to_string())?;
                Ok(None)
            }
            Err(error) => Err(error.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct TrayIntegration {
    handle: ksni::Handle<PulseTray>,
}

impl TrayIntegration {
    pub async fn start(commands: Sender<AppCommand>) -> Result<Self, String> {
        let icon = TrayIcon::load().map_err(|error| error.to_string())?;
        let handle = PulseTray {
            commands,
            paused: false,
            icon,
        }
        .spawn()
        .await
        .map_err(|e| e.to_string())?;
        Ok(Self { handle })
    }

    pub async fn set_paused(&self, paused: bool) {
        self.handle.update(move |tray| tray.paused = paused).await;
    }

    pub fn shutdown(&self) {
        // Sending shutdown is immediate; awaiting is only needed to observe D-Bus teardown.
        drop(self.handle.shutdown());
    }
}

pub struct PulseTray {
    commands: Sender<AppCommand>,
    paused: bool,
    icon: TrayIcon,
}

impl PulseTray {
    fn send(&self, command: AppCommand) {
        if let Err(error) = self.commands.try_send(command) {
            eprintln!("Pulse 无法处理托盘操作：{error}");
        }
    }

    fn item(label: &str, command: AppCommand) -> MenuItem<Self> {
        StandardItem {
            label: label.into(),
            activate: Box::new(move |tray: &mut Self| tray.send(command)),
            ..Default::default()
        }
        .into()
    }
}

impl Tray for PulseTray {
    fn id(&self) -> String {
        format!("{APP_ID}.Tray")
    }
    fn title(&self) -> String {
        "Pulse 系统监控".into()
    }
    fn icon_name(&self) -> String {
        self.icon.name.clone()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icon.pixmaps.clone()
    }
    fn activate(&mut self, _: i32, _: i32) {
        self.send(AppCommand::Show);
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Pulse".into(),
            description: if self.paused {
                "监控已暂停"
            } else {
                "本地资源监控 · 点击打开"
            }
            .into(),
            ..Default::default()
        }
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            Self::item("打开 Pulse", AppCommand::Show),
            MenuItem::Separator,
            Self::item(
                if self.paused {
                    "继续监控"
                } else {
                    "暂停监控"
                },
                AppCommand::ToggleSampling,
            ),
            Self::item("进程", AppCommand::Navigate(Page::Processes)),
            Self::item("设置", AppCommand::Navigate(Page::Settings)),
            Self::item("最小化窗口", AppCommand::Minimize),
            MenuItem::Separator,
            Self::item("退出 Pulse", AppCommand::Quit),
        ]
    }
    fn watcher_online(&self) {
        self.send(AppCommand::TrayAvailable(true));
    }
    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        eprintln!("Pulse 系统托盘暂不可用：{reason:?}");
        self.send(AppCommand::TrayAvailable(false));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_activation_and_menu_dispatch_application_commands() {
        let (sender, receiver) = async_channel::bounded(8);
        let mut tray = PulseTray {
            commands: sender,
            paused: false,
            icon: TrayIcon {
                name: String::new(),
                pixmaps: vec![ksni::Icon {
                    width: 32,
                    height: 32,
                    data: vec![0; 32 * 32 * 4],
                }],
            },
        };
        tray.activate(0, 0);
        assert_eq!(receiver.try_recv(), Ok(AppCommand::Show));
        let MenuItem::Standard(pause) = tray.menu().remove(2) else {
            panic!("pause menu item");
        };
        (pause.activate)(&mut tray);
        assert_eq!(receiver.try_recv(), Ok(AppCommand::ToggleSampling));
        tray.paused = true;
        let MenuItem::Standard(resume) = tray.menu().remove(2) else {
            panic!("resume menu item");
        };
        assert_eq!(resume.label, "继续监控");
        assert_eq!(tray.icon_pixmap()[0].data.len(), 4096);
        assert_eq!(tray.id(), format!("{APP_ID}.Tray"));
        assert!(tray.icon_name().is_empty());
    }
}
