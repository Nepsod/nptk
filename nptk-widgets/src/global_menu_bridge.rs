#[cfg(target_os = "linux")]
mod platform {
    use log::{error, warn};
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::ffi::CString;
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

    const MENU_OBJECT_PATH: &str = "/com/canonical/menu/1";
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
            let tx_for_thread = tx.clone();

            thread::Builder::new()
                .name("nptk-global-menu".into())
                .spawn(move || {
                    if let Err(err) = run(cmd_rx, evt_tx, tx_for_thread) {
                        error!("Global menu bridge thread exited: {err}");
                    }
                })
                .ok()?;

            Some(Self { tx, rx: evt_rx })
        }

        pub fn update_menu(&self, snapshot: MenuSnapshot) {
            let _ = self.tx.send(Command::UpdateMenu(snapshot));
        }

        pub fn set_window_id(&self, window_id: Option<u64>) {
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
        SetWindow(Option<u64>),
        RequestLayout(i32),
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
            self.revision = (self.revision.wrapping_add(1)).max(1);
        }

        fn layout_with(&self, parent_id: i32, depth: i32, _properties: Vec<&str>) -> SubMenuLayout {
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

            SubMenuLayout {
                id: parent_id,
                fields: {
                    let mut root_fields: HashMap<String, OwnedValue> = HashMap::new();
                    if parent_id == 0 {
                        // Root node should be minimal - only children-display property.
                        // libdbusmenu-qt treats any item-like properties (label, enabled, visible, type)
                        // as making it a menu item rather than a container.
                        root_fields.insert("children-display".into(), owned_value("menubar"));
                    }
                    root_fields
                },
                submenus,
            }
        }
    }

    struct MenuObject {
        state: Arc<Mutex<MenuState>>,
        evt_tx: Sender<BridgeEvent>,
        cmd_tx: Sender<Command>,
    }

    #[interface(name = "com.canonical.dbusmenu")]
    impl MenuObject {
        #[zbus(name = "AboutToShow")]
        async fn about_to_show(&self, _id: i32) -> bool {
            log::info!("DBusMenu.AboutToShow id={}", _id);
            // Return true if this node has (or may have) children to show
            let has_children = if _id == 0 {
                !self.state.lock().unwrap().entries.is_empty()
            } else {
                self
                    .state
                    .lock()
                    .unwrap()
                    .entries
                    .iter()
                    .any(|n| n.id == _id && !n.children.is_empty())
            };
            if has_children {
                // Ask the bridge loop to emit LayoutUpdated for this parent
                let _ = self.cmd_tx.send(Command::RequestLayout(_id));
            }
            has_children
        }

        #[zbus(name = "Event")]
        async fn event(
            &self,
            id: i32,
            event_id: &str,
            _data: OwnedValue,
            _timestamp: u32,
        ) {
            log::info!("DBusMenu.Event id={} event_id={}", id, event_id);
            if event_id == "clicked" {
                let _ = self.evt_tx.send(BridgeEvent::Activated(id));
            } else if event_id == "opened" {
                let _ = self.cmd_tx.send(Command::RequestLayout(id));
            } else if event_id == "about-to-show" {
                let _ = self.cmd_tx.send(Command::RequestLayout(id));
            }
        }

        #[zbus(name = "GetLayout")]
        async fn get_layout(
            &self,
            parent_id: i32,
            depth: i32,
            properties: Vec<&str>,
        ) -> (u32, SubMenuLayout) {
            let st = self.state.lock().unwrap();
            let props_debug = properties.clone();
            let layout = st.layout_with(parent_id, depth, properties);
            log::info!(
                "DBusMenu.GetLayout parent_id={} depth={} props={:?} revision={} entries_count={} submenus_count={}",
                parent_id,
                depth,
                props_debug,
                st.revision,
                st.entries.len(),
                layout.submenus.len()
            );
            (st.revision, layout)
        }

        #[zbus(name = "GetGroupProperties")]
        async fn get_group_properties(
            &self,
            ids: Vec<i32>,
            properties: Vec<String>,
        ) -> (u32, Vec<(i32, HashMap<String, OwnedValue>)>) {
            log::info!("DBusMenu.GetGroupProperties ids={:?} props={:?}", ids, properties);
            let st = self.state.lock().unwrap();
            let mut out: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
            for id in ids {
                if id == 0 {
                    let mut m: HashMap<String, OwnedValue> = HashMap::new();
                    let want_all = properties.is_empty();
                    // Root node should be minimal - only children-display property.
                    // libdbusmenu-qt treats any item-like properties (label, enabled, visible, type)
                    // as making it a menu item rather than a container.
                    if want_all || properties.iter().any(|p| p == "children-display") {
                        m.insert("children-display".into(), owned_value("menubar"));
                    }
                    // Explicitly do NOT include label, enabled, visible, or type for root node
                    out.push((id, m));
                    continue;
                }
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

        #[zbus(name = "GetProperty")]
        async fn get_property(&self, _id: i32, _name: &str) -> OwnedValue {
            log::info!("DBusMenu.GetProperty id={} name={}", _id, _name);
            // Root node is a pure container - only children-display property exists
            if _id == 0 {
                return match _name {
                    "children-display" => owned_value("menubar"),
                    // Root node has no item-like properties. Return empty string for all others
                    // to ensure it's treated as a pure container, not a menu item.
                    _ => owned_value(String::new()),
                };
            }
            if let Some(node) = find_node_by_id(&self.state.lock().unwrap().entries, _id) {
                if let Some(v) = node_property_value(node, _name) {
                    return v;
                }
            }
            owned_value(String::new())
        }

        #[zbus(signal)]
        #[zbus(name = "LayoutUpdated")]
        async fn layout_updated(
            emitter: &SignalEmitter<'_>,
            revision: u32,
            parent: i32,
        ) -> zbus::Result<()>;

        #[zbus(property)]
        #[zbus(name = "Status")]
        fn status(&self) -> &str {
            "normal"
        }

        #[zbus(property)]
        #[zbus(name = "Version")]
        fn version(&self) -> u32 {
            4
        }

        #[zbus(signal)]
        #[zbus(name = "ItemsPropertiesUpdated")]
        async fn items_properties_updated(
            emitter: &SignalEmitter<'_>,
            updated: Vec<(i32, HashMap<String, OwnedValue>)>,
            removed: Vec<(i32, Vec<String>)>,
        ) -> zbus::Result<()>;
    }

    fn run(cmd_rx: Receiver<Command>, evt_tx: Sender<BridgeEvent>, cmd_tx: Sender<Command>) -> ZbusResult<()> {
        // D-Bus well-known names must have elements that don't start with a digit.
        // Use a letter-prefixed instance component to incorporate the PID safely.
        let service_name = format!("com.nptk.menubar.app_{}", std::process::id());
        let state = Arc::new(Mutex::new(MenuState::default()));
        let menu_obj = MenuObject {
            state: state.clone(),
            evt_tx,
            cmd_tx,
        };

        let connection = ConnectionBuilder::session()?
            .name(WellKnownName::try_from(service_name.clone())?)?
            .serve_at(MENU_OBJECT_PATH, menu_obj)?
            .build()?;
        log::info!("Global menu DBus service '{}', object '{}'", service_name, MENU_OBJECT_PATH);

        let iface_ref = connection
            .object_server()
            .interface::<_, MenuObject>(MENU_OBJECT_PATH)?;
        let mut registrar = AppMenuRegistrar::new(&connection, service_name.clone());

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
                    } else {
                        log::info!("Global menu registered window id: {:?}", id);
                        // Set X11 window hints for Plasma appmenu discovery (X11/XWayland only)
                        if let Some(window_id) = id {
                            if let Err(err) = set_x11_appmenu_hints(window_id as u32, &service_name) {
                                warn!("Failed to set X11 appmenu hints: {err}");
                            }
                        }
                        // Nudge clients to query the layout after registration
                        // CRITICAL: We MUST run this even if entries are empty, so the root node (id=0)
                        // properties are sent. This prevents a race condition where SetWindow arrives
                        // before UpdateMenu - Plasma needs to know about the root container first.
                        let state_guard = state.lock().unwrap();
                        let revision = state_guard.revision;
                        drop(state_guard);
                        if let Err(err) = block_on(MenuObject::layout_updated(
                            iface_ref.signal_emitter(),
                            revision,
                            0,
                        )) {
                            warn!("Failed to emit layout update after registration: {err}");
                        }
                        // Also publish full properties so importers can seed their models
                        let state_guard = state.lock().unwrap();
                        let mut updates = flatten_properties_updates(&state_guard.entries);
                        // CRITICAL: Root node (id=0) must be a pure container with ONLY children-display.
                        // libdbusmenu-qt treats ANY item-like properties (label, enabled, visible, type)
                        // as making it a menu item rather than a container. This causes empty menus.
                        let mut root_map: HashMap<String, OwnedValue> = HashMap::new();
                        root_map.insert("children-display".into(), owned_value("menubar"));
                        // DO NOT include: label, enabled, visible, or type for root node
                        updates.push((0, root_map));
                        drop(state_guard);
                        let removed: Vec<(i32, Vec<String>)> = Vec::new();
                        if let Err(err) = block_on(MenuObject::items_properties_updated(
                            iface_ref.signal_emitter(),
                            updates,
                            removed,
                        )) {
                            warn!("Failed to emit initial items properties after registration: {err}");
                        }
                    }
                },
                Ok(Command::RequestLayout(parent)) => {
                    let st_guard = state.lock().unwrap();
                    let revision = st_guard.revision;
                    // Emit LayoutUpdated for this parent id
                    if let Err(err) = block_on(MenuObject::layout_updated(
                        iface_ref.signal_emitter(),
                        revision,
                        parent,
                    )) {
                        warn!("Failed to emit layout update for parent {parent}: {err}");
                    }
                    // Emit ItemsPropertiesUpdated for immediate children of this parent
                    let mut updates: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
                    if parent == 0 {
                        for n in &st_guard.entries {
                            updates.push((n.id, node_properties_map(n)));
                        }
                        // Do NOT emit LayoutUpdated for children here - the importer will call
                        // AboutToShow(id) or GetLayout(id, ...) when it needs the layout for a submenu.
                    } else if let Some(pnode) = find_node_by_id(&st_guard.entries, parent) {
                        for c in &pnode.children {
                            updates.push((c.id, node_properties_map(c)));
                        }
                    }
                    drop(st_guard);
                    let removed: Vec<(i32, Vec<String>)> = Vec::new();
                    if let Err(err) = block_on(MenuObject::items_properties_updated(
                        iface_ref.signal_emitter(),
                        updates,
                        removed,
                    )) {
                        warn!("Failed to emit items properties updated for parent {parent}: {err}");
                    }
                },
                Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {},
            }
            // Note: zbus blocking Connection processes incoming messages internally when serving.
        }

        Ok(())
    }

    struct AppMenuRegistrar<'a> {
        proxy: Proxy<'a>,
        current: Option<u64>,
        service: String,
    }

    impl<'a> AppMenuRegistrar<'a> {
        fn new(connection: &'a Connection, service: String) -> Self {
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
                service,
            }
        }

        fn set_window(&mut self, window_id: Option<u64>) -> ZbusResult<()> {
            if self.current == window_id {
                return Ok(());
            }

            if let Some(id) = window_id {
                let path = ObjectPath::try_from(MENU_OBJECT_PATH)?;
                // Always use 3-arg version to ensure service name is registered.
                // This helps Plasma resolve the menu even if it only has the unique bus name.
                let call3: ZbusResult<()> = self
                    .proxy
                    .call("RegisterWindow", &((id as u32), self.service.as_str(), path.clone()));
                if call3.is_err() {
                    // Fall back to 2-arg if 3-arg fails (for compatibility)
                    log::debug!("3-arg RegisterWindow failed, trying 2-arg");
                    let call2: ZbusResult<()> =
                        self.proxy.call("RegisterWindow", &((id as u32), path));
                    if call2.is_err() {
                        return Err(call2.unwrap_err());
                    }
                    log::debug!("Window registered with 2-arg RegisterWindow");
                } else {
                    log::debug!("Window registered with 3-arg RegisterWindow (service={})", self.service);
                }
            } else if let Some(id) = self.current.take() {
                let _: () = self.proxy.call("UnregisterWindow", &((id as u32),))?;
            }

            self.current = window_id;
            Ok(())
        }
    }

    #[derive(Clone, serde::Serialize, zbus::zvariant::Type)]
    struct SubMenuLayout {
        id: i32,
        fields: HashMap<String, OwnedValue>,
        submenus: Vec<OwnedValue>,
    }

    fn remote_node_to_owned_value(node: &RemoteMenuNode) -> OwnedValue {
        let mut fields: HashMap<String, OwnedValue> = HashMap::new();
        if !node.is_separator {
            let label = node.label.replace('_', "__");
            fields.insert("label".into(), owned_value(label));
        }
        fields.insert("enabled".into(), OwnedValue::from(node.enabled));
        fields.insert("visible".into(), OwnedValue::from(true));
        if node.is_separator {
            fields.insert("type".into(), owned_value("separator"));
        }
        if !node.children.is_empty() {
            fields.insert("children-display".into(), owned_value("submenu"));
        }
        if let Some(shortcut) = &node.shortcut {
            if let Some(seq) = encode_shortcut(shortcut) {
                fields.insert("shortcut".into(), owned_value(seq));
            }
        }

        let children: Vec<OwnedValue> = Vec::new();

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
        if !node.is_separator {
            let label = node.label.replace('_', "__");
            fields.insert("label".into(), owned_value(label));
        }
        fields.insert("enabled".into(), OwnedValue::from(node.enabled));
        fields.insert("visible".into(), OwnedValue::from(true));
        if node.is_separator {
            fields.insert("type".into(), owned_value("separator"));
        }
        if !node.children.is_empty() {
            fields.insert("children-display".into(), owned_value("submenu"));
            // DO NOT add type: "menu" - type is only for item types like "separator",
            // not for indicating that an item has children. children-display is sufficient.
        }
        if let Some(shortcut) = &node.shortcut {
            if let Some(seq) = encode_shortcut(shortcut) {
                fields.insert("shortcut".into(), owned_value(seq));
            }
        }

        // depth semantics:
        // -1 → recurse fully; 0 → no children; N≥1 → include children and recurse with N-1
        let recurse_children = depth < 0 || depth >= 1;
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
        if !node.is_separator {
            let label = node.label.replace('_', "__");
            props.insert("label".into(), owned_value(label));
        }
        props.insert("enabled".into(), OwnedValue::from(node.enabled));
        props.insert("visible".into(), OwnedValue::from(true));
        if node.is_separator {
            props.insert("type".into(), owned_value("separator"));
        }
        if !node.children.is_empty() {
            props.insert("children-display".into(), owned_value("submenu"));
            // DO NOT add type: "menu" - type is only for item types like "separator",
            // not for indicating that an item has children. children-display is sufficient.
        }
        if let Some(shortcut) = &node.shortcut {
            if let Some(seq) = encode_shortcut(shortcut) {
                props.insert("shortcut".into(), owned_value(seq));
            }
        }
        props
    }

    fn node_property_value(node: &RemoteMenuNode, name: &str) -> Option<OwnedValue> {
        match name {
            "label" if !node.is_separator => Some(owned_value(node.label.replace('_', "__"))),
            "enabled" => Some(OwnedValue::from(node.enabled)),
            "visible" => Some(OwnedValue::from(true)),
            "type" if node.is_separator => Some(owned_value("separator")),
            "children-display" if !node.children.is_empty() => Some(owned_value("submenu")),
            "shortcut" => node.shortcut.as_ref().and_then(|s| encode_shortcut(s.as_str())).map(owned_value),
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

    #[cfg(feature = "global-menu")]
    fn set_x11_appmenu_hints(window_id: u32, service_name: &str) -> Result<(), String> {
        use x11_dl::xlib::{Xlib, XA_STRING, PropModeReplace};

        let xlib = Xlib::open().map_err(|e| format!("Failed to load X11 library: {e}"))?;
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return Err("Failed to open X11 display".to_string());
            }

            // Get atoms for the KDE appmenu properties
            let service_name_atom_cstr = CString::new("_KDE_NET_WM_APPMENU_SERVICE_NAME")
                .map_err(|e| format!("Failed to create CString: {e}"))?;
            let object_path_atom_cstr = CString::new("_KDE_NET_WM_APPMENU_OBJECT_PATH")
                .map_err(|e| format!("Failed to create CString: {e}"))?;

            let service_name_atom = (xlib.XInternAtom)(
                display,
                service_name_atom_cstr.as_ptr(),
                0, // only_if_exists = false
            );
            let object_path_atom = (xlib.XInternAtom)(
                display,
                object_path_atom_cstr.as_ptr(),
                0, // only_if_exists = false
            );

            if service_name_atom == 0 || object_path_atom == 0 {
                (xlib.XCloseDisplay)(display);
                return Err("Failed to intern X11 atoms".to_string());
            }

            // Set _KDE_NET_WM_APPMENU_SERVICE_NAME property
            let service_name_cstr = CString::new(service_name)
                .map_err(|e| format!("Failed to create service name CString: {e}"))?;
            (xlib.XChangeProperty)(
                display,
                window_id as u64,
                service_name_atom,
                XA_STRING,
                8, // format: 8 bits per element
                PropModeReplace,
                service_name_cstr.as_ptr() as *const u8,
                service_name_cstr.as_bytes().len() as i32,
            );

            // Set _KDE_NET_WM_APPMENU_OBJECT_PATH property
            let object_path_cstr = CString::new(MENU_OBJECT_PATH)
                .map_err(|e| format!("Failed to create object path CString: {e}"))?;
            (xlib.XChangeProperty)(
                display,
                window_id as u64,
                object_path_atom,
                XA_STRING,
                8, // format: 8 bits per element
                PropModeReplace,
                object_path_cstr.as_ptr() as *const u8,
                object_path_cstr.as_bytes().len() as i32,
            );

            // Flush to ensure properties are set
            (xlib.XFlush)(display);
            (xlib.XCloseDisplay)(display);

            log::info!(
                "Set X11 appmenu hints: service={}, path={} on window {}",
                service_name,
                MENU_OBJECT_PATH,
                window_id
            );
        }

        Ok(())
    }

    #[cfg(not(feature = "global-menu"))]
    fn set_x11_appmenu_hints(_window_id: u32, _service_name: &str) -> Result<(), String> {
        Ok(())
    }

    // Encode "Ctrl+Shift+N" into [["Ctrl","Shift","N"]] per DBusMenu spec.
    fn encode_shortcut(s: &str) -> Option<Vec<Vec<String>>> {
        let parts: Vec<String> = s
            .split('+')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(vec![parts])
        }
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
