// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: app.rs
// @Software: resource-pulse

use crate::{
    collector::Collector,
    commands::{AppCommand, MenuGroup},
    desktop::{GlobalMenuIntegration, Instance, TrayIntegration},
    menu::MenuState,
    model::{Dashboard, Metric, SamplingInterval},
    preferences::{PreferenceStore, Preferences},
};
use pulse_ui::{
    button,
    gpui::{
        self, App, Context, Div, FocusHandle, KeyBinding, SharedString, Stateful, Subscription,
        Task, Window, actions, prelude::*,
    },
};

actions!(
    pulse,
    [
        ToggleSampling,
        CloseDetails,
        QuitApplication,
        MinimizeWindow,
        OpenSettings
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", ToggleSampling, Some("Pulse")),
        KeyBinding::new("escape", CloseDetails, Some("Pulse")),
        KeyBinding::new("ctrl-q", QuitApplication, Some("Pulse")),
        KeyBinding::new("ctrl-m", MinimizeWindow, Some("Pulse")),
        KeyBinding::new("ctrl-,", OpenSettings, Some("Pulse")),
    ]);
}

pub struct ResourcePulse {
    pub(super) state: Dashboard,
    pub(super) collector: Option<Collector>,
    sample_task: Option<Task<()>>,
    desktop_task: Option<Task<()>>,
    global_menu_watch: Option<Task<()>>,
    _menu_observer: Option<Subscription>,
    global_menu: Option<GlobalMenuIntegration>,
    pub(super) global_menu_available: bool,
    visibility: Option<Subscription>,
    tray: Option<TrayIntegration>,
    instance: Option<Instance>,
    preference_store: Option<PreferenceStore>,
    pub(super) preferences: Preferences,
    pub(super) preference_error: Option<&'static str>,
    pub(super) tray_available: bool,
    pub(super) desktop_error: Option<String>,
    pub(super) menu: Option<MenuGroup>,
    pub(super) about: bool,
    visible: bool,
    pub(super) error: Option<String>,
    pub(super) focus: FocusHandle,
}

impl ResourcePulse {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: Dashboard::default(),
            collector: None,
            sample_task: None,
            desktop_task: None,
            global_menu_watch: None,
            _menu_observer: Some(cx.observe_self(|this, cx| this.sync_global_menu(cx))),
            global_menu: None,
            global_menu_available: false,
            visibility: None,
            tray: None,
            instance: None,
            preference_store: None,
            preferences: Preferences::default(),
            preference_error: None,
            tray_available: false,
            desktop_error: None,
            menu: None,
            about: false,
            visible: true,
            error: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.load_preferences();
        let collector = match Collector::start() {
            Ok(collector) => collector,
            Err(error) => {
                eprintln!("Pulse 无法启动监控线程：{error}");
                self.error = Some("无法读取系统状态，请重新打开 Pulse。".into());
                return;
            }
        };
        let samples = collector.samples.clone();
        collector.set_interval(self.state.interval);
        self.collector = Some(collector);
        self.sample_task = Some(cx.spawn(async move |this, cx| {
            while let Ok(snapshot) = samples.recv().await {
                if this
                    .update(cx, |this, cx| {
                        this.state.ingest(snapshot);
                        if this.visible {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.error = Some("系统状态暂不可用，请重新打开 Pulse。".into());
                cx.notify();
            });
        }));
        self.visibility = Some(
            cx.observe_window_visibility(window, |this, visibility, _, cx| {
                this.set_visible(visibility.is_visible());
                if this.visible {
                    cx.notify();
                }
            }),
        );
        let view = cx.weak_entity();
        window.on_window_should_close(cx, move |window, cx| {
            view.update(cx, |this, cx| {
                if this.tray_available && this.preferences.close_to_tray {
                    this.execute_command(AppCommand::Minimize, window, cx);
                    false
                } else {
                    true
                }
            })
            .unwrap_or(true)
        });
    }

    pub fn attach_desktop(
        &mut self,
        sender: async_channel::Sender<AppCommand>,
        receiver: async_channel::Receiver<AppCommand>,
        instance: Option<Instance>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.instance = instance;
        let native_window = GlobalMenuIntegration::window_id(window);
        let menu_state = MenuState::new(&self.state, self.error.is_some());
        let window = window.window_handle();
        self.desktop_task = Some(cx.spawn(async move |this, cx| {
            if let Some(native_window) = native_window {
                match GlobalMenuIntegration::start(native_window, sender.clone(), menu_state).await
                {
                    Ok((global_menu, available)) => {
                        let commands = sender.clone();
                        if this
                            .update(cx, |this, cx| {
                                this.global_menu = Some(global_menu.clone());
                                this.global_menu_available = available;
                                this.global_menu_watch =
                                    Some(cx.background_executor().spawn(async move {
                                        if let Err(error) =
                                            global_menu.watch(commands.clone()).await
                                        {
                                            eprintln!("Pulse 全局菜单注册监听已停止：{error}");
                                            let _ = commands
                                                .send(AppCommand::GlobalMenuAvailable(false))
                                                .await;
                                        }
                                    }));
                                this.sync_global_menu(cx);
                                cx.notify();
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => eprintln!("Pulse 无法导出全局菜单：{error}"),
                }
            }
            let tray = TrayIntegration::start(sender).await;
            if this
                .update(cx, |this, cx| {
                    match tray {
                        Ok(tray) => {
                            this.tray = Some(tray);
                            this.tray_available = true;
                        }
                        Err(error) => {
                            eprintln!("Pulse 系统托盘不可用：{error}");
                            this.desktop_error =
                                Some("当前桌面不支持系统托盘，关闭窗口将退出 Pulse。".into());
                        }
                    }
                    cx.notify();
                })
                .is_err()
            {
                return;
            }
            while let Ok(command) = receiver.recv().await {
                if window
                    .update(cx, |_, window, cx| {
                        this.update(cx, |this, cx| this.execute_command(command, window, cx))
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    fn set_visible(&mut self, visible: bool) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        if let Some(collector) = &self.collector {
            collector.set_detail(visible);
        }
    }

    pub(super) fn execute_command(
        &mut self,
        command: AppCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu = None;
        match command {
            AppCommand::Show => {
                self.set_visible(true);
                self.focus.focus(window, cx);
                window.activate_window();
            }
            AppCommand::Navigate(page) => {
                self.state.navigate(page);
                self.set_visible(true);
                window.activate_window();
            }
            AppCommand::ToggleSampling => self.toggle_pause(cx),
            AppCommand::Interval(interval) => self.set_interval(interval),
            AppCommand::ClearHistory => self.state.history.clear(),
            AppCommand::CopySummary => cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                Metric::ALL
                    .into_iter()
                    .map(|metric| {
                        format!("{}: {}", metric.title(), metric.value(&self.state.latest))
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )),
            AppCommand::Minimize => {
                self.set_visible(false);
                window.minimize_window();
            }
            AppCommand::About => self.about = true,
            AppCommand::Quit => cx.quit(),
            AppCommand::TrayAvailable(available) => {
                self.tray_available = available;
                if !available {
                    window.activate_window();
                }
            }
            AppCommand::GlobalMenuAvailable(available) => self.global_menu_available = available,
        }
        cx.notify();
    }

    fn sync_global_menu(&self, cx: &Context<Self>) {
        let Some(menu) = self.global_menu.clone() else {
            return;
        };
        match menu.set_state(MenuState::new(&self.state, self.error.is_some())) {
            Ok(Some(revision)) => {
                cx.background_executor()
                    .spawn(async move {
                        if let Err(error) = menu.notify(revision).await {
                            eprintln!("Pulse 无法同步全局菜单：{error}");
                        }
                    })
                    .detach();
            }
            Ok(None) => {}
            Err(error) => eprintln!("Pulse 全局菜单状态不可用：{error}"),
        }
    }

    pub(super) fn control(
        &self,
        id: &'static str,
        text: impl Into<SharedString>,
        selected: bool,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        button(
            id,
            text,
            selected,
            cx.listener(move |this, _, _, cx| {
                action(this, cx);
                cx.notify();
            }),
        )
        .debug_selector(move || id.into())
    }

    pub(super) fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        if self.error.is_some() {
            return;
        }
        self.state.paused = !self.state.paused;
        if let Some(collector) = &self.collector {
            collector.set_paused(self.state.paused);
        }
        if let Some(tray) = self.tray.clone() {
            let paused = self.state.paused;
            cx.background_executor()
                .spawn(async move {
                    tray.set_paused(paused).await;
                })
                .detach();
        }
    }

    pub(super) fn set_interval(&mut self, interval: SamplingInterval) {
        if self.state.interval == interval {
            return;
        }
        self.state.interval = interval;
        if let Some(collector) = &self.collector {
            collector.set_interval(interval);
        }
        self.preferences.refresh_seconds = interval.seconds();
        self.save_preferences();
    }

    pub(super) fn set_close_to_tray(&mut self, enabled: bool) {
        if self.preferences.close_to_tray == enabled {
            return;
        }
        self.preferences.close_to_tray = enabled;
        self.save_preferences();
    }

    fn load_preferences(&mut self) {
        match PreferenceStore::for_current_user() {
            Ok(store) => {
                match store.load() {
                    Ok(preferences) => {
                        if let Ok(interval) = preferences.interval() {
                            self.state.interval = interval;
                        }
                        self.preferences = preferences;
                    }
                    Err(error) => {
                        eprintln!("Pulse 无法读取设置：{error}");
                        self.preference_error = Some("无法读取已保存的设置，暂时使用默认设置。");
                    }
                }
                self.preference_store = Some(store);
            }
            Err(error) => {
                eprintln!("Pulse 配置目录不可用：{error}");
                self.preference_error = Some("无法保存设置，请检查用户目录的访问权限。");
            }
        }
    }

    fn save_preferences(&mut self) {
        if let Some(store) = &self.preference_store {
            match store.save(self.preferences) {
                Ok(()) => self.preference_error = None,
                Err(error) => {
                    eprintln!("Pulse 无法保存设置：{error}");
                    self.preference_error = Some("设置未能保存，请检查用户目录的访问权限。");
                }
            }
        }
    }

    pub(super) fn sampling_status(&self) -> &'static str {
        if self.error.is_some() {
            "无法更新"
        } else if self.state.paused {
            "已暂停"
        } else if self.state.ready {
            "实时监控"
        } else {
            "正在连接"
        }
    }
}

impl Drop for ResourcePulse {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(tray) = &self.tray {
            tray.shutdown();
        }
    }
}
