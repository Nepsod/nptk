//! Plasma window management protocol integration for global menu support.
//!
//! This module implements the KDE AppMenu protocol (`org.kde.kwin.appmenu_manager`)
//! to allow Plasma's compositor to discover application menus on Wayland.
//!
//! **Note on winit compatibility**: Since winit now only supports X11 (Wayland support disabled),
//! winit windows are X11 windows and cannot be used with Wayland protocols. This module creates
//! its own Wayland connection for native Wayland windows. For winit windows, rely on
//! the registrar (`com.canonical.AppMenu.Registrar`) and app_id matching for menu discovery.
//!
//! This module is primarily useful for native Wayland implementations where you have
//! direct access to the `wl_surface` from your Wayland connection.
//!
//! See: https://wayland.app/protocols/kde-appmenu

#![cfg(all(target_os = "linux", feature = "global-menu"))]

use std::sync::{Arc, Mutex, OnceLock};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_surface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols_plasma::appmenu::client::{
    org_kde_kwin_appmenu, org_kde_kwin_appmenu_manager,
};

use nptk_core::platform::MenuInfoStorage;
mod wl_integration;

/// Plasma AppMenu client state.
struct PlasmaMenuState {
    menu_info: Arc<Mutex<Option<(String, String)>>>,
    appmenu_objects:
        Arc<Mutex<std::collections::HashMap<u32, org_kde_kwin_appmenu::OrgKdeKwinAppmenu>>>,
}

static PLASMA_CLIENT: OnceLock<Arc<Mutex<Option<PlasmaMenuClient>>>> = OnceLock::new();

struct PlasmaMenuClient {
    connection: Connection,
    queue_handle: QueueHandle<PlasmaMenuState>,
    state: Arc<Mutex<PlasmaMenuState>>,
    _event_queue: Mutex<EventQueue<PlasmaMenuState>>,
    appmenu_manager: Option<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager>,
}

impl PlasmaMenuClient {
    fn initialize() -> Result<Self, String> {
        // Only try to initialize if we're actually on a Wayland session
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return Err("Not on Wayland session (WAYLAND_DISPLAY not set)".to_string());
        }

        let connection = Connection::connect_to_env()
            .map_err(|e| format!("Failed to connect to Wayland display: {:?}", e))?;

        let (global_list, mut event_queue) = registry_queue_init::<PlasmaMenuState>(&connection)
            .map_err(|e| format!("Failed to init Wayland registry: {:?}", e))?;
        let queue_handle = event_queue.handle();

        let menu_info = Arc::new(Mutex::new(MenuInfoStorage::get()));
        let appmenu_objects = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let state = Arc::new(Mutex::new(PlasmaMenuState {
            menu_info: menu_info.clone(),
            appmenu_objects: appmenu_objects.clone(),
        }));

        let mut client_state = PlasmaMenuState {
            menu_info: menu_info.clone(),
            appmenu_objects: appmenu_objects.clone(),
        };

        // Perform initial roundtrip to get globals
        event_queue
            .roundtrip(&mut client_state)
            .map_err(|e| format!("Initial Wayland roundtrip failed: {:?}", e))?;

        // Bind to org.kde.kwin.appmenu_manager global if available
        // Try version 2 first (KWin supports it), fall back to version 1
        let appmenu_manager = match global_list
            .bind::<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager, _, _>(
            &queue_handle,
            1..=2,
            (),
        ) {
            Ok(manager) => {
                let version = manager.version();
                log::info!("Bound to org.kde.kwin.appmenu_manager version {}", version);
                Some(manager)
            },
            Err(wayland_client::globals::BindError::NotPresent) => {
                log::debug!("org.kde.kwin.appmenu_manager not available (not on KWin?)");
                None
            },
            Err(err) => {
                log::warn!("Failed to bind org.kde.kwin.appmenu_manager: {:?}", err);
                None
            },
        };

        Ok(Self {
            connection,
            queue_handle,
            state,
            _event_queue: Mutex::new(event_queue),
            appmenu_manager,
        })
    }

    fn update_menu_info(&self) {
        if self.appmenu_manager.is_some() {
            let state_guard = self.state.lock().unwrap();
            let menu_info_guard = state_guard.menu_info.lock().unwrap();
            if let Some((ref service, ref path)) = *menu_info_guard {
                log::debug!("Menu info available: service={}, path={}", service, path);
                // Menu info will be set when we call set_appmenu_for_surface
            }
        }
    }

    /// Set the application menu for a Wayland surface.
    ///
    /// This should be called when a window surface is created and menu info is available.
    pub fn set_appmenu_for_surface(&self, surface: &wl_surface::WlSurface) -> Result<(), String> {
        let Some(ref manager) = self.appmenu_manager else {
            return Err("AppMenu manager not available".to_string());
        };

        let state_guard = self.state.lock().unwrap();
        let menu_info_guard = state_guard.menu_info.lock().unwrap();
        let Some((ref service, ref path)) = *menu_info_guard else {
            return Err("Menu info not available yet".to_string());
        };

        // Create an appmenu object for this surface
        let appmenu = manager.create(surface, &self.queue_handle, ());
        let surface_id = surface.id().protocol_id();

        // Set the menu address
        appmenu.set_address(service.clone(), path.clone());

        // Store the appmenu object
        let mut appmenu_objects = state_guard.appmenu_objects.lock().unwrap();
        appmenu_objects.insert(surface_id, appmenu);

        log::info!(
            "Set application menu for surface {}: service={}, path={}",
            surface_id,
            service,
            path
        );

        Ok(())
    }

    fn dispatch_events(&self) -> Result<(), String> {
        let mut event_queue = self._event_queue.lock().unwrap();
        let mut state = self.state.lock().unwrap();
        event_queue
            .dispatch_pending(&mut *state)
            .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;
        Ok(())
    }
}

/// Initialize the Plasma AppMenu protocol client.
///
/// This should be called when the application starts and a window is created.
/// It will bind to the `org.kde.kwin.appmenu_manager` global if available.
pub fn initialize() -> Result<(), String> {
    let client = PlasmaMenuClient::initialize()?;
    PLASMA_CLIENT
        .set(Arc::new(Mutex::new(Some(client))))
        .map_err(|_| "Plasma client already initialized".to_string())?;

    // Update menu info if available
    if let Some(ref client) = *PLASMA_CLIENT.get().unwrap().lock().unwrap() {
        client.update_menu_info();
    }

    Ok(())
}

/// Set the application menu for a Wayland surface.
///
/// This should be called when a window surface is created and menu info is available.
/// The surface should be a `wl_surface` from the Wayland connection.
pub fn set_appmenu_for_surface(surface: &wl_surface::WlSurface) -> Result<(), String> {
    let client_guard = PLASMA_CLIENT.get().ok_or("Plasma client not initialized")?;
    let client = client_guard.lock().unwrap();
    if let Some(ref client) = *client {
        client.set_appmenu_for_surface(surface)?;
    } else {
        return Err("Plasma client not initialized".to_string());
    }
    Ok(())
}

/// Set application menu properties via Plasma's Wayland window management protocol.
///
/// This is a convenience function that initializes the Plasma client and stores
/// menu info for later use when surfaces are created.
pub fn set_appmenu_properties(service_name: &str) -> Result<(), String> {
    // Store menu info for later use
    MenuInfoStorage::set(
        service_name.to_string(),
        "/com/canonical/menu/1".to_string(),
    );

    // Notify VGI's Wayland client about the menu update
    // This will set appmenu for all existing surfaces
    wl_integration::notify_wl_client_menu_update();

    // Try to initialize the Plasma client (for standalone use, separate from VGI's client)
    if let Err(err) = initialize() {
        log::debug!(
            "Failed to initialize Plasma window management client: {err} (may not be on Plasma)"
        );
    } else {
        log::info!("Plasma window management protocol client initialized");
    }

    Ok(())
}

/// Update the menu info when it changes.
///
/// This should be called whenever the menu service name or object path changes.
pub fn update_menu_info() {
    if let Some(ref client) = *PLASMA_CLIENT.get().unwrap().lock().unwrap() {
        // Update the stored menu info
        if let Some((service, path)) = MenuInfoStorage::get() {
            let state_guard = client.state.lock().unwrap();
            let mut menu_info_guard = state_guard.menu_info.lock().unwrap();
            *menu_info_guard = Some((service, path));
        }
        client.update_menu_info();
    }
}

/// Dispatch pending events from the Plasma window management protocol.
///
/// This should be called periodically (e.g., in the main event loop) to process
/// window creation events and other protocol messages from the compositor.
pub fn dispatch_events() -> Result<(), String> {
    if let Some(ref client) = *PLASMA_CLIENT.get().unwrap().lock().unwrap() {
        client.dispatch_events()?;
    }
    Ok(())
}

/// Check if the Plasma client is initialized.
pub fn is_initialized() -> bool {
    PLASMA_CLIENT.get().is_some() && PLASMA_CLIENT.get().unwrap().lock().unwrap().is_some()
}

impl
    Dispatch<
        wayland_client::protocol::wl_registry::WlRegistry,
        wayland_client::globals::GlobalListContents,
    > for PlasmaMenuState
{
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_registry::WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &wayland_client::globals::GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Registry events are handled by GlobalList
    }
}

impl Dispatch<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager, ()> for PlasmaMenuState {
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager,
        _event: org_kde_kwin_appmenu_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The appmenu manager doesn't send events to clients
    }
}

impl Dispatch<org_kde_kwin_appmenu::OrgKdeKwinAppmenu, ()> for PlasmaMenuState {
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_appmenu::OrgKdeKwinAppmenu,
        _event: org_kde_kwin_appmenu::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The appmenu object doesn't send events to clients
    }
}
