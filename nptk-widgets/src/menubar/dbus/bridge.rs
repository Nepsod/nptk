//! DBus menu bridge implementation.

use log::{error, warn};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use zbus::block_on;
use zbus::blocking::connection::Builder as ConnectionBuilder;
use zbus::names::WellKnownName;
use zbus::zvariant::OwnedValue;
use zbus::Result as ZbusResult;

use crate::menubar::common::{platform, x11};
use nptk_core::platform::MenuInfoStorage;
use crate::menubar::plasma;

use super::menu_object::MenuObject;
use super::registrar::AppMenuRegistrar;
use super::types::{MenuSnapshot, MenuState, flatten_properties_updates, node_properties_map, owned_value, properties_index};

const MENU_OBJECT_PATH: &str = "/com/canonical/menu/1";

/// Events emitted by the bridge.
#[derive(Clone)]
pub enum BridgeEvent {
    Activated(i32),
    /// Emitted when a global menu importer is detected (e.g., GetLayout called for root menu)
    ImporterDetected,
}

/// Commands sent to the bridge thread.
pub(crate) enum Command {
    UpdateMenu(MenuSnapshot),
    SetWindow(Option<u64>),
    RequestLayout(i32),
    Shutdown,
}

/// The global menu bridge.
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

fn run(cmd_rx: Receiver<Command>, evt_tx: Sender<BridgeEvent>, cmd_tx: Sender<Command>) -> ZbusResult<()> {
    // D-Bus well-known names must have elements that don't start with a digit.
    // 
    // Service name strategy:
    // - For winit: Use static "nptk.menubar" (matches app_id "nptk" via Plasma's heuristic)
    // - For native Wayland: Use "com.nptk.app.menubar_p{pid}" (protocol-based, PID needed for uniqueness)
    //
    // Detect platform using Platform::detect() to correctly identify native Wayland vs winit/X11
    let is_native_wayland = platform::is_native_wayland();
    
    let service_name = if is_native_wayland {
        // Native Wayland: use PID-based name (protocol works directly, so uniqueness is important)
        format!("com.nptk.app.menubar_p{}", std::process::id())
    } else {
        // Winit mode: try service name that starts with app_id for better Plasma matching
        // Try "nptk.menubar" - starts with app_id "nptk" exactly
        // D-Bus names must have at least one dot, so "nptk.menubar" is valid
        "nptk.menubar".to_string()
    };
    
    log::info!(
        "Global menu service name: '{}' (platform: {})",
        service_name,
        if is_native_wayland { "native Wayland" } else { "winit" }
    );
    
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

    // Store menu info for platform integrations
    MenuInfoStorage::set(service_name.clone(), MENU_OBJECT_PATH.to_string());

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
                    if is_native_wayland {
                        // Native Wayland platform
                        if let Some(window_id) = id {
                            if window_id == 1 {
                                log::info!(
                                    "Global menu registered on Wayland with dummy window ID: {:?} (service={}, path={})",
                                    id,
                                    service_name,
                                    MENU_OBJECT_PATH
                                );
                                log::warn!(
                                    "Using dummy window ID on Wayland. Plasma may not be able to match the window to the menu. \
                                     Ensure the window's app_id is set to 'com.nptk.app' to match the menu service pattern 'com.nptk.app.menubar_p*'."
                                );
                            } else {
                                log::info!(
                                    "Global menu registered on Wayland with surface ID: {} (service={}, path={})",
                                    window_id,
                                    service_name,
                                    MENU_OBJECT_PATH
                                );
                                log::info!(
                                    "For Plasma to discover the menu, the window's app_id should be 'com.nptk.app' to match the menu service pattern 'com.nptk.app.menubar_p*'. \
                                     The menu service name is '{}'.",
                                    service_name
                                );
                            }
                        }
                        log::debug!(
                            "On Wayland, Plasma's compositor discovers menus through window properties and app_id matching."
                        );
                    } else {
                        // X11 or XWayland (winit with X11)
                        log::info!("Global menu registered window id: {:?}", id);
                    }
                    
                    // Set X11 window hints for Plasma appmenu discovery
                    // Always try to set X11 hints if we have a window ID - the function will fail gracefully
                    // if we're not on X11. This handles both pure X11 and XWayland cases.
                    // On native Wayland, this will fail but that's okay - we use Wayland protocols instead.
                    if let Some(window_id) = id {
                        if let Err(err) = x11::set_appmenu_hints(window_id as u32, &service_name, MENU_OBJECT_PATH) {
                            log::debug!("Failed to set X11 appmenu hints (may not be on X11/XWayland): {err}");
                        } else {
                            log::debug!("Successfully set X11 appmenu hints on window {}", window_id);
                        }
                    }
                    
                    // On native Wayland, also try to set menu properties via Plasma window management
                    if is_native_wayland {
                        if let Err(err) = plasma::set_appmenu_properties(&service_name) {
                            log::debug!("Failed to set Wayland appmenu properties (may not be on Plasma): {err}");
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
                } else if let Some(pnode) = super::types::find_node_by_id(&st_guard.entries, parent) {
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

