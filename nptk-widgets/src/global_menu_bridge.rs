#[cfg(target_os = "linux")]
mod platform {
    use log::{error, warn};
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use zbus::block_on;
    use zbus::blocking::connection::Builder as ConnectionBuilder;
    use zbus::blocking::{Connection, Proxy};
    use zbus::interface;
    use zbus::names::WellKnownName;
    use zbus::object_server::SignalEmitter;
    use zbus::zvariant::{ObjectPath, OwnedValue, Structure, Value};
    use zbus::Result as ZbusResult;

    const MENU_OBJECT_PATH: &str = "/com/nptk/MenuBar";
    const REGISTRAR_BUS: &str = "com.canonical.AppMenu.Registrar";
    const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";
    const REGISTRAR_INTERFACE: &str = "com.canonical.AppMenu.Registrar";

    #[derive(Clone)]
    pub enum BridgeEvent {
        Activated(i32),
    }

    pub struct Bridge {
        tx: Sender<Command>,
        rx: Receiver<BridgeEvent>,
    }

    impl Bridge {
        pub fn start() -> Option<Self> {
            let (tx, cmd_rx) = mpsc::channel();
            let (evt_tx, evt_rx) = mpsc::channel();

            thread::Builder::new()
                .name("nptk-global-menu".into())
                .spawn(move || {
                    if let Err(err) = run(cmd_rx, evt_tx) {
                        error!("Global menu bridge thread exited: {err}");
                    }
                })
                .ok()?;

            Some(Self { tx, rx: evt_rx })
        }

        pub fn update_menu(&self, snapshot: MenuSnapshot) {
            let _ = self.tx.send(Command::UpdateMenu(snapshot));
        }

        pub fn set_window_id(&self, window_id: Option<u32>) {
            let _ = self.tx.send(Command::SetWindow(window_id));
        }

        pub fn poll_events(&self) -> Vec<BridgeEvent> {
            let mut events = Vec::new();
            while let Ok(event) = self.rx.try_recv() {
                events.push(event);
            }
            events
        }
    }

    impl Drop for Bridge {
        fn drop(&mut self) {
            let _ = self.tx.send(Command::Shutdown);
        }
    }

    #[derive(Clone)]
    pub struct MenuSnapshot {
        pub entries: Vec<RemoteMenuNode>,
    }

    #[derive(Clone)]
    pub struct RemoteMenuNode {
        pub id: i32,
        pub label: String,
        pub enabled: bool,
        pub is_separator: bool,
        pub shortcut: Option<String>,
        pub children: Vec<RemoteMenuNode>,
    }

    enum Command {
        UpdateMenu(MenuSnapshot),
        SetWindow(Option<u32>),
        Shutdown,
    }

    #[derive(Default)]
    struct MenuState {
        revision: u32,
        entries: Vec<RemoteMenuNode>,
    }

    impl MenuState {
        fn replace(&mut self, snapshot: MenuSnapshot) {
            self.entries = snapshot.entries;
            self.revision = self.revision.wrapping_add(1).max(1);
        }

        fn layout_with(&self, parent_id: i32, depth: i32, properties: Vec<&str>) -> MenuLayout {
            // - parent_id = 0 → top-level entries
            // - depth: -1 = full recursion, 0 = only this node, >0 recurse that many levels
            let submenus: Vec<OwnedValue> = if parent_id == 0 {
                self.entries
                    .iter()
                    .map(|n| build_owned_value_recursive(n, depth))
                    .collect()
            } else if let Some(node) = find_node_by_id(&self.entries, parent_id) {
                node.children
                    .iter()
                    .map(|n| build_owned_value_recursive(n, depth))
                    .collect()
            } else {
                Vec::new()
            };

            MenuLayout {
                id: 0,
                fields: SubMenuLayout {
                    id: parent_id,
                    fields: HashMap::new(),
                    submenus,
                },
            }
        }
    }

    struct MenuObject {
        state: Arc<Mutex<MenuState>>,
        evt_tx: Sender<BridgeEvent>,
    }

    #[interface(name = "com.canonical.dbusmenu")]
    impl MenuObject {
        async fn about_to_show(&self, _id: i32) -> bool {
            false
        }

        async fn event(
            &self,
            id: i32,
            event_id: &str,
            _data: OwnedValue,
            _timestamp: u32,
        ) {
            if event_id == "clicked" {
                let _ = self.evt_tx.send(BridgeEvent::Activated(id));
            }
        }

        async fn get_layout(
            &self,
            parent_id: i32,
            depth: i32,
            properties: Vec<&str>,
        ) -> MenuLayout {
            self.state.lock().unwrap().layout_with(parent_id, depth, properties)
        }

        async fn get_group_properties(
            &self,
            ids: Vec<i32>,
            properties: Vec<String>,
        ) -> (u32, Vec<(i32, HashMap<String, OwnedValue>)>) {
            let st = self.state.lock().unwrap();
            let mut out: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
            for id in ids {
                if let Some(node) = find_node_by_id(&st.entries, id) {
                    if properties.is_empty() {
                        let map = node_properties_map(node);
                        out.push((id, map));
                    } else {
                        let mut map: HashMap<String, OwnedValue> = HashMap::new();
                        for p in properties.iter() {
                            if let Some(v) = node_property_value(node, p.as_str()) {
                                map.insert(p.clone(), v);
                            }
                        }
                        out.push((id, map));
                    }
                }
            }
            (st.revision, out)
        }

        async fn get_property(&self, _id: i32, _name: &str) -> OwnedValue {
            // Minimal fallback
            if let Some(node) = find_node_by_id(&self.state.lock().unwrap().entries, _id) {
                if let Some(v) = node_property_value(node, _name) {
                    return v;
                }
            }
            owned_value(0u32)
        }

        #[zbus(signal)]
        async fn layout_updated(
            emitter: &SignalEmitter<'_>,
            revision: u32,
            parent: i32,
        ) -> zbus::Result<()>;

        #[zbus(property)]
        fn status(&self) -> &str {
            "normal"
        }

        #[zbus(property)]
        fn version(&self) -> u32 {
            4
        }

        #[zbus(signal)]
        async fn items_properties_updated(
            emitter: &SignalEmitter<'_>,
            updated: Vec<(i32, HashMap<String, OwnedValue>)>,
            removed: Vec<(i32, Vec<String>)>,
        ) -> zbus::Result<()>;
    }

    fn run(cmd_rx: Receiver<Command>, evt_tx: Sender<BridgeEvent>) -> ZbusResult<()> {
        let service_name = format!("org.nptk.AppMenu.{}", std::process::id());
        let state = Arc::new(Mutex::new(MenuState::default()));
        let menu_obj = MenuObject {
            state: state.clone(),
            evt_tx,
        };

        let connection = ConnectionBuilder::session()?
            .name(WellKnownName::try_from(service_name.clone())?)?
            .serve_at(MENU_OBJECT_PATH, menu_obj)?
            .build()?;

        let iface_ref = connection
            .object_server()
            .interface::<_, MenuObject>(MENU_OBJECT_PATH)?;
        let mut registrar = AppMenuRegistrar::new(&connection);

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(16)) {
                Ok(Command::UpdateMenu(snapshot)) => {
                    // Diff properties before/after to emit a tighter ItemsPropertiesUpdated.
                    let prev_index = properties_index(&state.lock().unwrap().entries);
                    state.lock().unwrap().replace(snapshot);
                    let guard = state.lock().unwrap();
                    let next_index = properties_index(&guard.entries);
                    let mut updates: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
                    for (id, props) in next_index.iter() {
                        match prev_index.get(id) {
                            Some(prev) if prev == props => {},
                            _ => updates.push((*id, props.clone())),
                        }
                    }
                    let revision = guard.revision;
                    if let Err(err) = block_on(MenuObject::layout_updated(
                        iface_ref.signal_emitter(),
                        revision,
                        0,
                    )) {
                        warn!("Failed to emit layout update: {err}");
                    }
                    let removed: Vec<(i32, Vec<String>)> = Vec::new();
                    if let Err(err) = block_on(MenuObject::items_properties_updated(
                        iface_ref.signal_emitter(),
                        updates,
                        removed,
                    )) {
                        warn!("Failed to emit items properties updated: {err}");
                    }
                },
                Ok(Command::SetWindow(id)) => {
                    if let Err(err) = registrar.set_window(id) {
                        warn!("Failed to register global menu window: {err}");
                    }
                },
                Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {},
            }
        }

        Ok(())
    }

    struct AppMenuRegistrar<'a> {
        proxy: Proxy<'a>,
        current: Option<u32>,
    }

    impl<'a> AppMenuRegistrar<'a> {
        fn new(connection: &'a Connection) -> Self {
            let proxy = Proxy::new(
                connection,
                REGISTRAR_BUS,
                REGISTRAR_PATH,
                REGISTRAR_INTERFACE,
            )
            .expect("Failed to connect to AppMenu registrar");
            Self {
                proxy,
                current: None,
            }
        }

        fn set_window(&mut self, window_id: Option<u32>) -> ZbusResult<()> {
            if self.current == window_id {
                return Ok(());
            }

            if let Some(id) = window_id {
                let path = ObjectPath::try_from(MENU_OBJECT_PATH)?;
                let _: () = self.proxy.call("RegisterWindow", &(id, path))?;
            } else if let Some(id) = self.current.take() {
                let _: () = self.proxy.call("UnregisterWindow", &(id,))?;
            }

            self.current = window_id;
            Ok(())
        }
    }

    #[derive(Clone, serde::Serialize, zbus::zvariant::Type)]
    struct MenuLayout {
        id: u32,
        fields: SubMenuLayout,
    }

    #[derive(Clone, serde::Serialize, zbus::zvariant::Type)]
    struct SubMenuLayout {
        id: i32,
        fields: HashMap<String, OwnedValue>,
        submenus: Vec<OwnedValue>,
    }

    fn remote_node_to_owned_value(node: &RemoteMenuNode) -> OwnedValue {
        let mut fields: HashMap<String, OwnedValue> = HashMap::new();
        let label = if node.is_separator {
            node.label.clone()
        } else {
            node.label.replace('_', "__")
        };
        fields.insert("label".into(), owned_value(label));
        fields.insert("enabled".into(), OwnedValue::from(node.enabled));
        fields.insert("visible".into(), OwnedValue::from(true));
        if node.is_separator {
            fields.insert("type".into(), owned_value("separator"));
        }
        if !node.children.is_empty() {
            fields.insert("children-display".into(), owned_value("submenu"));
        }
        if let Some(shortcut) = &node.shortcut {
            fields.insert("shortcut".into(), owned_value(shortcut.clone()));
        }

        let children: Vec<OwnedValue> = node
            .children
            .iter()
            .map(remote_node_to_owned_value)
            .collect();

        owned_value(Structure::from((node.id, fields, children)))
    }

    fn find_node_by_id<'a>(roots: &'a [RemoteMenuNode], id: i32) -> Option<&'a RemoteMenuNode> {
        for n in roots {
            if n.id == id {
                return Some(n);
            }
            if let Some(found) = find_node_by_id(&n.children, id) {
                return Some(found);
            }
        }
        None
    }

    fn build_owned_value_recursive(node: &RemoteMenuNode, depth: i32) -> OwnedValue {
        let mut fields: HashMap<String, OwnedValue> = HashMap::new();
        let label = if node.is_separator {
            node.label.clone()
        } else {
            node.label.replace('_', "__")
        };
        fields.insert("label".into(), owned_value(label));
        fields.insert("enabled".into(), OwnedValue::from(node.enabled));
        fields.insert("visible".into(), OwnedValue::from(true));
        if node.is_separator {
            fields.insert("type".into(), owned_value("separator"));
        }
        if !node.children.is_empty() {
            fields.insert("children-display".into(), owned_value("submenu"));
        }
        if let Some(shortcut) = &node.shortcut {
            fields.insert("shortcut".into(), owned_value(shortcut.clone()));
        }

        let recurse_children = depth != 0;
        let next_depth = if depth < 0 { -1 } else { depth.saturating_sub(1) };
        let children: Vec<OwnedValue> = if recurse_children {
            node.children
                .iter()
                .map(|c| build_owned_value_recursive(c, next_depth))
                .collect()
        } else {
            Vec::new()
        };

        owned_value(Structure::from((node.id, fields, children)))
    }

    fn node_properties_map(node: &RemoteMenuNode) -> HashMap<String, OwnedValue> {
        let mut props: HashMap<String, OwnedValue> = HashMap::new();
        let label = if node.is_separator {
            node.label.clone()
        } else {
            node.label.replace('_', "__")
        };
        props.insert("label".into(), owned_value(label));
        props.insert("enabled".into(), OwnedValue::from(node.enabled));
        props.insert("visible".into(), OwnedValue::from(true));
        if node.is_separator {
            props.insert("type".into(), owned_value("separator"));
        }
        if !node.children.is_empty() {
            props.insert("children-display".into(), owned_value("submenu"));
        }
        if let Some(shortcut) = &node.shortcut {
            props.insert("shortcut".into(), owned_value(shortcut.clone()));
        }
        props
    }

    fn node_property_value(node: &RemoteMenuNode, name: &str) -> Option<OwnedValue> {
        match name {
            "label" => {
                let label = if node.is_separator { node.label.clone() } else { node.label.replace('_', "__") };
                Some(owned_value(label))
            },
            "enabled" => Some(OwnedValue::from(node.enabled)),
            "visible" => Some(OwnedValue::from(true)),
            "type" if node.is_separator => Some(owned_value("separator")),
            "children-display" if !node.children.is_empty() => Some(owned_value("submenu")),
            "shortcut" => node.shortcut.as_ref().map(|s| owned_value(s.clone())),
            _ => None,
        }
    }

    fn flatten_properties_updates(
        roots: &[RemoteMenuNode],
    ) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        fn recurse<'a>(
            node: &'a RemoteMenuNode,
            acc: &mut Vec<(i32, HashMap<String, OwnedValue>)>,
        ) {
            acc.push((node.id, node_properties_map(node)));
            for c in &node.children {
                recurse(c, acc);
            }
        }

        let mut out = Vec::new();
        for n in roots {
            recurse(n, &mut out);
        }
        out
    }

    fn properties_index(roots: &[RemoteMenuNode]) -> HashMap<i32, HashMap<String, OwnedValue>> {
        let mut index: HashMap<i32, HashMap<String, OwnedValue>> = HashMap::new();
        fn recurse(node: &RemoteMenuNode, index: &mut HashMap<i32, HashMap<String, OwnedValue>>) {
            index.insert(node.id, node_properties_map(node));
            for c in &node.children {
                recurse(c, index);
            }
        }
        for n in roots {
            recurse(n, &mut index);
        }
        index
    }

    fn owned_value<T>(value: T) -> OwnedValue
    where
        Value<'static>: From<T>,
    {
        OwnedValue::try_from(Value::from(value)).expect("value conversion")
    }
}

#[cfg(target_os = "linux")]
pub use platform::{Bridge, BridgeEvent, MenuSnapshot, RemoteMenuNode};

#[cfg(not(target_os = "linux"))]
mod platform {
    #[derive(Clone)]
    pub enum BridgeEvent {
        Activated(i32),
    }

    #[derive(Clone)]
    pub struct MenuSnapshot {
        pub entries: Vec<RemoteMenuNode>,
    }

    #[derive(Clone)]
    pub struct RemoteMenuNode {
        pub id: i32,
        pub label: String,
        pub enabled: bool,
        pub is_separator: bool,
        pub shortcut: Option<String>,
        pub children: Vec<RemoteMenuNode>,
    }

    pub struct Bridge;

    impl Bridge {
        pub fn start() -> Option<Self> {
            None
        }

        pub fn update_menu(&self, _snapshot: MenuSnapshot) {}

        pub fn set_window_id(&self, _window_id: Option<u32>) {}

        pub fn poll_events(&self) -> Vec<BridgeEvent> {
            Vec::new()
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub use platform::{Bridge, BridgeEvent, MenuSnapshot, RemoteMenuNode};
