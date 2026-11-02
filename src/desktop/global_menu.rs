// @Time    : 2026/11/2 00:00
// @Author  : fzf
// @FileName: global_menu.rs
// @Software: resource-pulse

use crate::{
    commands::{AppCommand, MenuGroup},
    menu::{Entry, MenuState},
};
use async_channel::Sender;
use futures_lite::StreamExt;
use pulse_ui::gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};
use x11rb::{
    connection::Connection as _,
    protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode},
    wrapper::ConnectionExt as _,
};
use zbus::{
    Connection, fdo,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedValue, Str, StructureBuilder, Value},
};

const REGISTRAR: &str = "com.canonical.AppMenu.Registrar";
const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";
const MENU_PATH: &str = "/io/github/fzf/Pulse/MenuBar";
type Properties = HashMap<String, OwnedValue>;
type Layout = (i32, Properties, Vec<Value<'static>>);

struct SharedMenu {
    state: Mutex<MenuState>,
    revision: AtomicU32,
    commands: Sender<AppCommand>,
}

struct MenuServer(Arc<SharedMenu>);

fn invalid(message: impl Into<String>) -> fdo::Error {
    fdo::Error::InvalidArgs(message.into())
}

fn string(value: impl Into<String>) -> OwnedValue {
    Str::from(value.into()).into()
}

fn entry(id: i32, state: MenuState) -> Option<Entry> {
    if id < 100 {
        return None;
    }
    let group = *MenuGroup::ALL.get((id / 100 - 1) as usize)?;
    state.entries(group).into_iter().nth((id % 100) as usize)
}

fn properties(id: i32, state: MenuState, names: &[String]) -> fdo::Result<Properties> {
    let mut props = Properties::new();
    props.insert("visible".into(), true.into());
    props.insert("enabled".into(), true.into());
    if id == 0 {
        props.insert("children-display".into(), string("submenu"));
    } else if let Some(group) = (1..=4)
        .contains(&id)
        .then(|| MenuGroup::ALL[(id - 1) as usize])
    {
        props.insert("label".into(), string(group.title()));
        props.insert("children-display".into(), string("submenu"));
    } else {
        let item = entry(id, state).ok_or_else(|| invalid("unknown menu item"))?;
        props.insert("label".into(), string(item.label));
        props.insert("enabled".into(), item.enabled.into());
        if matches!(
            item.command,
            AppCommand::Navigate(_) | AppCommand::Interval(_)
        ) {
            props.insert("toggle-type".into(), string("radio"));
            props.insert("toggle-state".into(), i32::from(item.checked).into());
        }
        let shortcut: Option<Vec<Vec<&str>>> = match item.shortcut {
            "Ctrl+Q" => Some(vec![vec!["Control", "q"]]),
            "Ctrl+M" => Some(vec![vec!["Control", "m"]]),
            "Ctrl+," => Some(vec![vec!["Control", "comma"]]),
            "Space" => Some(vec![vec!["space"]]),
            _ => None,
        };
        if let Some(shortcut) = shortcut {
            props.insert(
                "shortcut".into(),
                Value::new(shortcut)
                    .try_into_owned()
                    .map_err(|e| invalid(e.to_string()))?,
            );
        }
    }
    if !names.is_empty() {
        props.retain(|name, _| names.contains(name));
    }
    Ok(props)
}

fn layout(id: i32, depth: i32, names: &[String], state: MenuState) -> fdo::Result<Layout> {
    let props = properties(id, state, names)?;
    let children = if depth == 0 {
        Vec::new()
    } else {
        let ids: Vec<_> = if id == 0 {
            (1..=4).collect()
        } else if (1..=4).contains(&id) {
            (0..state.entries(MenuGroup::ALL[(id - 1) as usize]).len())
                .map(|index| id * 100 + index as i32)
                .collect()
        } else {
            Vec::new()
        };
        ids.into_iter()
            .map(|child| {
                let (id, props, children) =
                    layout(child, if depth < 0 { -1 } else { depth - 1 }, names, state)?;
                let structure = StructureBuilder::new()
                    .add_field(id)
                    .add_field(props)
                    .add_field(children)
                    .build()
                    .map_err(|e| invalid(e.to_string()))?;
                Ok(Value::from(structure))
            })
            .collect::<fdo::Result<Vec<_>>>()?
    };
    Ok((id, props, children))
}

impl MenuServer {
    fn snapshot(&self) -> fdo::Result<MenuState> {
        self.0
            .state
            .lock()
            .map(|state| *state)
            .map_err(|_| fdo::Error::Failed("menu state unavailable".into()))
    }
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl MenuServer {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> fdo::Result<(u32, Layout)> {
        Ok((
            self.0.revision.load(Ordering::Acquire),
            layout(
                parent_id,
                recursion_depth,
                &property_names,
                self.snapshot()?,
            )?,
        ))
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<String>,
    ) -> fdo::Result<Vec<(i32, Properties)>> {
        let state = self.snapshot()?;
        let ids = if ids.is_empty() {
            let mut all = vec![0];
            for (index, group) in MenuGroup::ALL.into_iter().enumerate() {
                let parent = index as i32 + 1;
                all.push(parent);
                all.extend(
                    (0..state.entries(group).len()).map(|index| parent * 100 + index as i32),
                );
            }
            all
        } else {
            ids
        };
        Ok(ids
            .into_iter()
            .filter_map(|id| {
                properties(id, state, &property_names)
                    .ok()
                    .map(|props| (id, props))
            })
            .collect())
    }

    fn get_property(&self, id: i32, name: String) -> fdo::Result<OwnedValue> {
        properties(id, self.snapshot()?, &[])?
            .remove(&name)
            .ok_or_else(|| invalid("unknown menu property"))
    }

    fn event(
        &self,
        id: i32,
        event_id: &str,
        _data: OwnedValue,
        _timestamp: u32,
    ) -> fdo::Result<()> {
        let item =
            entry(id, self.snapshot()?).ok_or_else(|| invalid("unknown actionable menu item"))?;
        if event_id == "clicked" && item.enabled {
            self.0
                .commands
                .try_send(item.command)
                .map_err(|_| fdo::Error::Failed("application command queue unavailable".into()))?;
        }
        Ok(())
    }

    fn event_group(&self, events: Vec<(i32, String, OwnedValue, u32)>) -> Vec<i32> {
        events
            .into_iter()
            .filter_map(|(id, event, data, timestamp)| {
                self.event(id, &event, data, timestamp).err().map(|_| id)
            })
            .collect()
    }

    fn about_to_show(&self, id: i32) -> fdo::Result<bool> {
        properties(id, self.snapshot()?, &[])?;
        Ok(false)
    }

    fn about_to_show_group(&self, ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (
            Vec::new(),
            ids.into_iter()
                .filter(|id| self.about_to_show(*id).is_err())
                .collect(),
        )
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        4
    }
    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    #[zbus(signal)]
    async fn layout_updated(
        emitter: &SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct GlobalMenuIntegration {
    connection: Connection,
    shared: Arc<SharedMenu>,
    window: u32,
}

impl GlobalMenuIntegration {
    pub fn window_id(window: &Window) -> Option<u32> {
        match HasWindowHandle::window_handle(window).ok()?.as_raw() {
            RawWindowHandle::Xlib(handle) => u32::try_from(handle.window).ok(),
            RawWindowHandle::Xcb(handle) => Some(handle.window.get()),
            _ => None,
        }
    }

    pub async fn start(
        window: u32,
        commands: Sender<AppCommand>,
        state: MenuState,
    ) -> Result<(Self, bool), String> {
        let shared = Arc::new(SharedMenu {
            state: Mutex::new(state),
            revision: AtomicU32::new(1),
            commands,
        });
        let connection = zbus::connection::Builder::session()
            .map_err(|e| e.to_string())?
            .serve_at(MENU_PATH, MenuServer(shared.clone()))
            .map_err(|e| e.to_string())?
            .build()
            .await
            .map_err(|e| e.to_string())?;
        let integration = Self {
            connection,
            shared,
            window,
        };
        let available = integration.register().await.is_ok();
        Ok((integration, available))
    }

    async fn register(&self) -> Result<(), String> {
        let service = self
            .connection
            .unique_name()
            .ok_or("global menu connection has no unique name")?;
        publish_window_menu(self.window, service.as_str())?;
        let registrar = zbus::Proxy::new(&self.connection, REGISTRAR, REGISTRAR_PATH, REGISTRAR)
            .await
            .map_err(|e| e.to_string())?;
        registrar
            .call::<_, _, ()>(
                "RegisterWindow",
                &(
                    self.window,
                    ObjectPath::try_from(MENU_PATH).map_err(|e| e.to_string())?,
                ),
            )
            .await
            .map_err(|e| e.to_string())
    }

    pub fn set_state(&self, next: MenuState) -> Result<Option<u32>, String> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| "menu state unavailable")?;
        if *state == next {
            return Ok(None);
        }
        *state = next;
        Ok(Some(
            self.shared
                .revision
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1),
        ))
    }

    pub async fn notify(&self, revision: u32) -> Result<(), String> {
        let emitter = SignalEmitter::new(&self.connection, MENU_PATH).map_err(|e| e.to_string())?;
        MenuServer::layout_updated(&emitter, revision, 0)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn watch(self, commands: Sender<AppCommand>) -> Result<(), String> {
        let proxy = zbus::fdo::DBusProxy::new(&self.connection)
            .await
            .map_err(|e| e.to_string())?;
        let mut changes = proxy
            .receive_name_owner_changed_with_args(&[(0, REGISTRAR)])
            .await
            .map_err(|e| e.to_string())?;
        while let Some(change) = changes.next().await {
            let args = change.args().map_err(|e| e.to_string())?;
            let available = args.new_owner().is_some() && self.register().await.is_ok();
            if commands
                .send(AppCommand::GlobalMenuAvailable(available))
                .await
                .is_err()
            {
                break;
            }
        }
        Ok(())
    }
}

fn publish_window_menu(window: u32, service: &str) -> Result<(), String> {
    let (connection, _) = x11rb::connect(None).map_err(|e| e.to_string())?;
    for (name, value) in [
        ("_KDE_NET_WM_APPMENU_SERVICE_NAME", service),
        ("_KDE_NET_WM_APPMENU_OBJECT_PATH", MENU_PATH),
    ] {
        let atom = connection
            .intern_atom(false, name.as_bytes())
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .atom;
        connection
            .change_property8(
                PropMode::REPLACE,
                window,
                atom,
                AtomEnum::STRING,
                value.as_bytes(),
            )
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
    }
    connection.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SamplingInterval;
    use zbus::zvariant::Type;

    #[test]
    fn layout_matches_dbusmenu_wire_signature_and_has_four_groups() {
        assert_eq!(Layout::SIGNATURE.to_string(), "(ia{sv}av)");
        let (_, _, children) = layout(0, -1, &[], MenuState::default()).expect("root layout");
        assert_eq!(children.len(), 4);
        assert!(layout(999, -1, &[], MenuState::default()).is_err());
        assert!(
            layout(0, 0, &[], MenuState::default())
                .expect("shallow root")
                .2
                .is_empty()
        );
    }

    #[test]
    fn global_menu_dispatches_commands_and_tracks_disabled_items() {
        let (commands, receiver) = async_channel::bounded(8);
        let shared = Arc::new(SharedMenu {
            state: Mutex::new(MenuState::default()),
            revision: AtomicU32::new(1),
            commands,
        });
        let server = MenuServer(shared.clone());
        server
            .event(300, "clicked", 0_i32.into(), 0)
            .expect("pause");
        assert_eq!(receiver.try_recv(), Ok(AppCommand::ToggleSampling));
        server
            .event(304, "clicked", 0_i32.into(), 0)
            .expect("disabled clear");
        assert!(receiver.try_recv().is_err());
        let mut state = shared.state.lock().expect("state");
        state.interval = SamplingInterval::Five;
        state.paused = true;
        drop(state);
        assert_eq!(
            String::try_from(server.get_property(300, "label".into()).expect("label"))
                .expect("string"),
            "继续监控"
        );
        assert_eq!(
            i32::try_from(
                server
                    .get_property(303, "toggle-state".into())
                    .expect("selected interval")
            )
            .expect("integer"),
            1
        );
    }
}
