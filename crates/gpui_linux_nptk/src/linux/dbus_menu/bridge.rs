// SPDX-License-Identifier: LGPL-3.0-only
//! DBus menu bridge implementation.
//!
//! This module uses `zbus::blocking` for simplicity in handling D-Bus messages.
//! While `nptk` is moving towards async, the global menu integration currently relies
//! on a dedicated thread and blocking I/O for D-Bus communication to ensure responsiveness
//! independent of the main UI loop and to simplify the synchronous D-Bus interface.
//!
//! Future refactoring could move this to `zbus::connection` (async) and integrate it
//! into the main async runtime, but for now, the threaded blocking approach is intentional
//! and isolated.

use log::{error, warn};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use zbus::block_on;
use zbus::blocking::connection::Builder as ConnectionBuilder;
use zbus::names::WellKnownName;
use zbus::zvariant::OwnedValue;
use zbus::Result as ZbusResult;

use super::common;
use super::menu_object::MenuObject;
use super::registrar::AppMenuRegistrar;
use super::types::{
    flatten_properties_updates, node_properties_map, owned_value, properties_index, MenuSnapshot,
    MenuState,
};

const MENU_OBJECT_PATH: &str = "/com/canonical/menu/1";

/// Events emitted by the bridge.
#[derive(Clone)]
pub enum BridgeEvent {
    Activated(i32),
}

/// Commands sent to the bridge thread.
pub(crate) enum Command {
    UpdateMenu(MenuSnapshot),
    SetWindow(Option<u64>),
    RequestLayout(i32),
    Activated(i32),
    Shutdown,
}

/// The global menu bridge.
pub struct Bridge {
    tx: Sender<Command>,
}

impl Bridge {
    pub fn start(on_event: Arc<dyn Fn(BridgeEvent) + Send + Sync>) -> Option<Self> {
        let (tx, cmd_rx) = mpsc::channel();
        let tx_for_thread = tx.clone();

        thread::Builder::new()
            .name("nptk-global-menu".into())
            .spawn(move || {
                if let Err(err) = run(cmd_rx, on_event, tx_for_thread) {
                    error!("Global menu bridge thread exited: {err}");
                }
            })
            .ok()?;

        Some(Self { tx })
    }

    pub fn update_menu(&self, snapshot: MenuSnapshot) {
        let _ = self.tx.send(Command::UpdateMenu(snapshot));
    }

    pub fn set_window_id(&self, window_id: Option<u64>) {
        let _ = self.tx.send(Command::SetWindow(window_id));
    }


}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
    }
}

fn run(
    cmd_rx: Receiver<Command>,
    on_event: Arc<dyn Fn(BridgeEvent) + Send + Sync>,
    cmd_tx: Sender<Command>,
) -> ZbusResult<()> {
    // D-Bus well-known names must have elements that don't start with a digit.
    //
    // PID-based name avoids collisions when multiple NPTK apps export menus on the session bus.
    let service_name = format!("com.nptk.app.menubar_p{}", std::process::id());

    log::info!("Global menu service name: '{}'", service_name);

    let state = Arc::new(Mutex::new(MenuState::default()));
    let menu_obj = MenuObject {
        state: state.clone(),
        cmd_tx: cmd_tx.clone(),
    };

    let connection = ConnectionBuilder::session()?
        .name(WellKnownName::try_from(service_name.clone())?)?
        .serve_at(MENU_OBJECT_PATH, menu_obj)?
        .build()?;
    log::info!(
        "Global menu DBus service '{}', object '{}'",
        service_name,
        MENU_OBJECT_PATH
    );
    common::MenuInfoStorage::set(service_name.clone(), MENU_OBJECT_PATH.to_string());

    let iface_ref = connection
        .object_server()
        .interface::<_, MenuObject>(MENU_OBJECT_PATH)?;
    let mut registrar = AppMenuRegistrar::new(&connection, service_name.clone()).ok();
    let emit_request_layout = |parent: i32| {
        let (revision, updates) = {
            let state_guard = state.lock().unwrap();
            let revision = state_guard.revision;
            // Collect immediate children properties for this parent.
            let mut updates: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
            if parent == 0 {
                for node in &state_guard.entries {
                    updates.push((node.id, node_properties_map(node)));
                }
            } else if let Some(parent_node) = super::types::find_node_by_id(&state_guard.entries, parent)
            {
                for child in &parent_node.children {
                    updates.push((child.id, node_properties_map(child)));
                }
            }
            (revision, updates)
        };

        log::info!(
            "DBusMenu.RequestLayout parent={} revision={} updated_items={}",
            parent,
            revision,
            updates.len()
        );

        // Emit LayoutUpdated for this parent id.
        // IMPORTANT: Do not hold the state mutex while emitting signals,
        // importer clients may re-enter via GetLayout/GetProperty.
        if let Err(err) = block_on(MenuObject::layout_updated(
            iface_ref.signal_context(),
            revision,
            parent,
        )) {
            warn!("Failed to emit layout update for parent {parent}: {err}");
        }
        let removed: Vec<(i32, Vec<String>)> = Vec::new();
        if let Err(err) = block_on(MenuObject::items_properties_updated(
            iface_ref.signal_context(),
            updates,
            removed,
        )) {
            warn!("Failed to emit items properties updated for parent {parent}: {err}");
        }
    };
    let mut pending_commands = VecDeque::new();

    loop {
        while let Ok(command) = cmd_rx.try_recv() {
            pending_commands.push_back(command);
        }
        coalesce_pending_request_layout_commands(&mut pending_commands);

        let command = if let Some(activated_position) = pending_commands
            .iter()
            .position(|command| matches!(command, Command::Activated(_)))
        {
            pending_commands
                .remove(activated_position)
                .expect("activated command should exist")
        } else if let Some(request_layout_position) = pending_commands
            .iter()
            .position(|command| matches!(command, Command::RequestLayout(_)))
        {
            pending_commands
                .remove(request_layout_position)
                .expect("request layout command should exist")
        } else if let Some(command) = pending_commands.pop_front() {
            command
        } else {
            match cmd_rx.recv_timeout(Duration::from_millis(16)) {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
            }
        };

        match command {
            Command::UpdateMenu(snapshot) => {
                let root_entries_count = snapshot.entries.len();
                // Diff properties before/after to emit a tighter ItemsPropertiesUpdated.
                let (revision, updates) = {
                    let prev_index = properties_index(&state.lock().unwrap().entries);
                    state.lock().unwrap().replace(snapshot);
                    let guard = state.lock().unwrap();
                    let next_index = properties_index(&guard.entries);
                    let mut updates: Vec<(i32, HashMap<String, OwnedValue>)> = Vec::new();
                    for (id, props) in next_index {
                        match prev_index.get(&id) {
                            Some(prev) if prev == &props => {}
                            _ => updates.push((id, props)),
                        }
                    }
                    (guard.revision, updates)
                };
                log::info!(
                    "DBusMenu.UpdateMenu revision={} root_entries={} changed_items={}",
                    revision,
                    root_entries_count,
                    updates.len()
                );
                if let Err(err) = block_on(MenuObject::layout_updated(
                    iface_ref.signal_context(),
                    revision,
                    0,
                )) {
                    warn!("Failed to emit layout update: {err}");
                }
                let removed: Vec<(i32, Vec<String>)> = Vec::new();
                if let Err(err) = block_on(MenuObject::items_properties_updated(
                    iface_ref.signal_context(),
                    updates,
                    removed,
                )) {
                    warn!("Failed to emit items properties updated: {err}");
                }
            },
            Command::SetWindow(id) => {
                let registration_result = if let Some(registrar) = registrar.as_mut() {
                    registrar.set_window(id)
                } else {
                    log::debug!("Skipping RegisterWindow; AppMenu registrar unavailable");
                    Ok(false)
                };

                match registration_result {
                    Err(err) => warn!("Failed to register global menu window: {err}"),
                    Ok(false) => {
                        log::debug!("Skipping registration layout nudge; window registration unchanged");
                    },
                    Ok(true) => {
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

                    if let Some(window_id) = id {
                        if common::platform::is_wayland_session() {
                            log::trace!(
                                "Skipping X11 appmenu hints (Wayland session; id {window_id} is not an X window)"
                            );
                        } else if window_id != 0 {
                            if let Err(error) = common::x11::set_appmenu_hints(
                                window_id as u32,
                                &service_name,
                                MENU_OBJECT_PATH,
                            ) {
                                log::debug!("Failed to set X11 appmenu hints: {error}");
                            }
                        }
                    }

                    // Only nudge layout when SetWindow arrives before the first UpdateMenu.
                    // UpdateMenu already emits LayoutUpdated/ItemsPropertiesUpdated; repeating
                    // that here causes redundant Plasma importer work and can freeze the app.
                    let should_emit_registration_layout = {
                        let state_guard = state.lock().unwrap();
                        state_guard.revision == 0
                    };
                    if should_emit_registration_layout {
                        let revision = {
                            let state_guard = state.lock().unwrap();
                            state_guard.revision
                        };
                        if let Err(err) = block_on(MenuObject::layout_updated(
                            iface_ref.signal_context(),
                            revision,
                            0,
                        )) {
                            warn!("Failed to emit layout update after registration: {err}");
                        }
                        let mut updates = {
                            let state_guard = state.lock().unwrap();
                            flatten_properties_updates(&state_guard.entries)
                        };
                        let mut root_map: HashMap<String, OwnedValue> = HashMap::new();
                        root_map.insert("children-display".into(), owned_value("menubar"));
                        updates.push((0, root_map));
                        let removed: Vec<(i32, Vec<String>)> = Vec::new();
                        if let Err(err) = block_on(MenuObject::items_properties_updated(
                            iface_ref.signal_context(),
                            updates,
                            removed,
                        )) {
                            warn!(
                                "Failed to emit initial items properties after registration: {err}"
                            );
                        }
                    } else if state.lock().unwrap().revision > 0 {
                        // Menu was already published via UpdateMenu; nudge importers after
                        // Wayland appmenu binding and registrar know the window.
                        emit_request_layout(0);
                    }
                    },
                }
            },
            Command::RequestLayout(parent) => {
                emit_request_layout(parent);
            },
            Command::Activated(menu_item_id) => {
                (on_event)(BridgeEvent::Activated(menu_item_id));
            },
            Command::Shutdown => break,
        }
        // Note: zbus blocking Connection processes incoming messages internally when serving.
    }

    Ok(())
}

fn coalesce_pending_request_layout_commands(pending_commands: &mut VecDeque<Command>) {
    let mut latest_request_layout_by_parent: std::collections::HashMap<i32, ()> =
        std::collections::HashMap::new();
    let mut other_commands = VecDeque::new();

    for command in pending_commands.drain(..) {
        match command {
            Command::RequestLayout(parent) => {
                latest_request_layout_by_parent.insert(parent, ());
            }
            other => other_commands.push_back(other),
        }
    }

    for parent in latest_request_layout_by_parent.keys() {
        other_commands.push_back(Command::RequestLayout(*parent));
    }

    *pending_commands = other_commands;
}
