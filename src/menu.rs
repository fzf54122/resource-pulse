// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: menu.rs
// @Software: resource-pulse

use crate::{
    commands::{AppCommand, MenuGroup},
    model::{Dashboard, Page, SamplingInterval},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuState {
    pub page: Page,
    pub paused: bool,
    pub interval: SamplingInterval,
    pub has_history: bool,
    pub ready: bool,
    pub failed: bool,
}

impl MenuState {
    pub fn new(state: &Dashboard, failed: bool) -> Self {
        Self {
            page: state.page,
            paused: state.paused,
            interval: state.interval,
            has_history: !state.history.is_empty(),
            ready: state.ready,
            failed,
        }
    }
}
pub struct Entry {
    pub id: &'static str,
    pub label: String,
    pub shortcut: &'static str,
    pub command: AppCommand,
    pub enabled: bool,
    pub checked: bool,
}

impl Entry {
    fn new(
        id: &'static str,
        label: impl Into<String>,
        shortcut: &'static str,
        command: AppCommand,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            shortcut,
            command,
            enabled: true,
            checked: false,
        }
    }
}

impl MenuState {
    pub fn entries(&self, group: MenuGroup) -> Vec<Entry> {
        match group {
            MenuGroup::Application => vec![
                Entry::new("menu-about", "关于 Pulse", "", AppCommand::About),
                Entry::new(
                    "menu-settings",
                    "设置",
                    "Ctrl+,",
                    AppCommand::Navigate(Page::Settings),
                ),
                Entry::new("menu-quit", "退出 Pulse", "Ctrl+Q", AppCommand::Quit),
            ],
            MenuGroup::View => {
                let mut entries = Page::ALL
                    .into_iter()
                    .zip([
                        "menu-overview",
                        "menu-processes",
                        "menu-history",
                        "menu-preferences",
                    ])
                    .map(|(page, id)| {
                        let mut entry =
                            Entry::new(id, page.title(), "", AppCommand::Navigate(page));
                        entry.checked = self.page == page;
                        entry
                    })
                    .collect::<Vec<_>>();
                let mut copy = Entry::new("menu-copy", "复制系统摘要", "", AppCommand::CopySummary);
                copy.enabled = self.ready;
                entries.push(copy);
                entries
            }
            MenuGroup::Sampling => {
                let mut toggle = Entry::new(
                    "menu-toggle",
                    if self.paused {
                        "继续监控"
                    } else {
                        "暂停监控"
                    },
                    "Space",
                    AppCommand::ToggleSampling,
                );
                toggle.enabled = !self.failed;
                let mut entries = vec![toggle];
                entries.extend(
                    SamplingInterval::ALL
                        .into_iter()
                        .zip(["menu-interval-1", "menu-interval-2", "menu-interval-5"])
                        .map(|(interval, id)| {
                            let mut entry = Entry::new(
                                id,
                                format!("每 {} 秒更新", interval.seconds()),
                                "",
                                AppCommand::Interval(interval),
                            );
                            entry.checked = self.interval == interval;
                            entry
                        }),
                );
                let mut clear =
                    Entry::new("menu-clear", "清空历史记录", "", AppCommand::ClearHistory);
                clear.enabled = self.has_history;
                entries.push(clear);
                entries
            }
            MenuGroup::Window => vec![
                Entry::new(
                    "menu-minimize",
                    "最小化窗口",
                    "Ctrl+M",
                    AppCommand::Minimize,
                ),
                Entry::new("menu-show", "显示主窗口", "", AppCommand::Show),
            ],
        }
    }
}
