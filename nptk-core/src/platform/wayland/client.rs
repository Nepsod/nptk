#![cfg(target_os = "linux")]

//! Wayland client connection management and event loop.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use wayland_client::backend::WaylandError;
use wayland_client::globals::{registry_queue_init, GlobalList};
use wayland_client::protocol::wl_surface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

use super::globals::WaylandGlobals;
use super::shell::WaylandClientState;
use super::surface::WaylandSurfaceInner;

#[cfg(feature = "global-menu")]
use wayland_protocols_plasma::appmenu::client::org_kde_kwin_appmenu;

/// Singleton Wayland client used by all native Wayland surfaces.
pub struct WaylandClient {
    connection: Connection,
    queue_handle: QueueHandle<WaylandClientState>,
    globals: WaylandGlobals,
    shared: Arc<SharedState>,
    loop_data: Mutex<(EventQueue<WaylandClientState>, WaylandClientState)>,
}

pub(crate) struct SharedState {
    surfaces: Mutex<HashMap<u32, Weak<WaylandSurfaceInner>>>,
    focused_surface_key: Mutex<Option<u32>>,
    #[cfg(feature = "global-menu")]
    appmenu_objects: Mutex<HashMap<u32, org_kde_kwin_appmenu::OrgKdeKwinAppmenu>>,
    // Map callback IDs to surface keys for frame callback lookup
    callback_to_surface: Mutex<HashMap<u32, u32>>,
}

static WAYLAND_CLIENT: OnceLock<Arc<WaylandClient>> = OnceLock::new();

impl WaylandClient {
    pub fn instance() -> Arc<WaylandClient> {
        WAYLAND_CLIENT
            .get_or_init(|| Arc::new(Self::initialize().expect("Failed to init Wayland client")))
            .clone()
    }

    pub fn connection(&self) -> Connection {
        self.connection.clone()
    }

    pub fn queue_handle(&self) -> QueueHandle<WaylandClientState> {
        self.queue_handle.clone()
    }

    pub fn globals(&self) -> WaylandGlobals {
        self.globals.clone()
    }

    pub fn register_surface(&self, surface: &Arc<WaylandSurfaceInner>) {
        let mut map = self.shared.surfaces.lock().unwrap();
        let key = surface.surface_key();
        log::trace!("Wayland register_surface key={}", key);
        map.insert(key, Arc::downgrade(surface));
    }

    pub(crate) fn register_callback(&self, callback_id: u32, surface_key: u32) {
        let mut callback_map = self.shared.callback_to_surface.lock().unwrap();
        callback_map.insert(callback_id, surface_key);
    }

    #[cfg(feature = "global-menu")]
    /// Set appmenu for a Wayland surface with explicit menu info.
    /// This is called by the public appmenu API.
    pub(crate) fn set_appmenu_for_surface_with_info(
        &self,
        surface: &wl_surface::WlSurface,
        service: String,
        path: String,
    ) -> Result<(), String> {
        let Some(ref manager) = self.globals.appmenu_manager else {
            return Err("AppMenu manager not available".to_string());
        };
        
        let surface_id = surface.id().protocol_id();
        
        // Check if we already have an appmenu for this surface
        let mut appmenu_objects = self.shared.appmenu_objects.lock().unwrap();
        if appmenu_objects.contains_key(&surface_id) {
            log::debug!("Appmenu already set for surface {}, updating address", surface_id);
            // Update the existing appmenu
            let appmenu = appmenu_objects.get(&surface_id).unwrap();
            appmenu.set_address(service.clone(), path.clone());
        } else {
            // Create a new appmenu object for this surface
            let appmenu = manager.create(surface, &self.queue_handle, ());
            
            // Set the menu address
            appmenu.set_address(service.clone(), path.clone());
            
            // Store the appmenu object to keep it alive
            appmenu_objects.insert(surface_id, appmenu);
            
            log::info!(
                "Created and set application menu for surface {}: service={}, path={}",
                surface_id,
                service,
                path
            );
        }
        drop(appmenu_objects);
        
        // Flush the connection to ensure the compositor receives the message
        if let Err(err) = self.flush() {
            log::warn!("Failed to flush Wayland connection after setting appmenu: {err}");
        }
        
        // Dispatch any pending events
        if let Err(err) = self.dispatch_pending() {
            log::warn!("Failed to dispatch Wayland events after setting appmenu: {err}");
        }
        
        Ok(())
    }
    
    #[cfg(feature = "global-menu")]
    /// Update appmenu for all existing surfaces when menu info changes.
    /// This is called by the menubar module when menu info is updated.
    pub(crate) fn update_appmenu_for_all_surfaces(
        &self,
        service: String,
        path: String,
    ) {
        let surfaces_map = self.shared.surfaces.lock().unwrap();
        let surface_keys: Vec<u32> = surfaces_map.keys().copied().collect();
        let surface_count = surface_keys.len();
        drop(surfaces_map);
        
        log::info!("Attempting to set appmenu for {} existing surface(s) after menu info update", surface_count);
        if surface_count == 0 {
            log::debug!("No surfaces registered yet, appmenu will be set when surface is created");
        } else {
            for surface_key in surface_keys {
                if let Some(surface) = self.shared.get_surface(surface_key) {
                    log::debug!("Found surface {}, attempting to set appmenu after menu info update", surface_key);
                    if let Err(err) = self.set_appmenu_for_surface_with_info(surface.wl_surface(), service.clone(), path.clone()) {
                        log::warn!("Failed to set appmenu for surface {} after menu info update: {err}", surface_key);
                    } else {
                        log::info!("Successfully set appmenu for surface {} after menu info update", surface_key);
                    }
                } else {
                    log::debug!("Surface {} not found (may have been dropped)", surface_key);
                }
            }
        }
    }

    pub fn wait_for_initial_configure(&self, surface_key: u32) -> Result<(), String> {
        const INITIAL_CONFIGURE_TIMEOUT: Duration = Duration::from_secs(2);
        let start = Instant::now();

        loop {
            if let Some(surface) = self.shared.get_surface(surface_key) {
                if surface.has_acknowledged_initial_configure() {
                    return Ok(());
                }
            } else {
                return Err(format!(
                    "Wayland surface {} dropped before initial configure",
                    surface_key
                ));
            }

            {
                let mut data = self.loop_data.lock().unwrap();
                let (event_queue, state) = &mut *data;
                event_queue.roundtrip(state).map_err(|e| {
                    format!(
                        "Wayland roundtrip failed while waiting for configure: {:?}",
                        e
                    )
                })?;
            }

            if start.elapsed() >= INITIAL_CONFIGURE_TIMEOUT {
                return Err(format!(
                    "Timed out waiting for initial configure on surface {}",
                    surface_key
                ));
            }
        }
    }

    pub fn unregister_surface(&self, surface_key: u32) {
        let mut map = self.shared.surfaces.lock().unwrap();
        map.remove(&surface_key);
        
        // Also remove the appmenu object when surface is unregistered
        #[cfg(feature = "global-menu")]
        {
            let mut appmenu_objects = self.shared.appmenu_objects.lock().unwrap();
            appmenu_objects.remove(&surface_key);
        }
    }

    pub fn dispatch_pending(&self) -> Result<(), String> {
        let mut data = self.loop_data.lock().unwrap();
        let (event_queue, state) = &mut *data;
        log::trace!("Wayland dispatch_pending queue_ptr={:p}", event_queue);
        
        // Log keyboard state before dispatch
        if let Some(ref keyboard) = self.globals.keyboard {
            let focused = *self.shared.focused_surface_key.lock().unwrap();
            log::debug!("Wayland dispatch_pending: keyboard={:?}, focused_surface={:?}", keyboard.id(), focused);
        } else {
            log::debug!("Wayland dispatch_pending: no keyboard available");
        }

        // First process anything that might already be queued.
        event_queue
            .dispatch_pending(state)
            .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;

        // Attempt to pull in fresh events without blocking the UI thread on the socket.
        loop {
            match event_queue.prepare_read() {
                Some(guard) => {
                    event_queue
                        .flush()
                        .map_err(|e| format!("Failed to flush Wayland queue: {:?}", e))?;
                    match guard.read() {
                        Ok(_) => {},
                        Err(WaylandError::Io(ref err)) if err.kind() == ErrorKind::WouldBlock => {
                            break;
                        },
                        Err(err) => {
                            return Err(format!("Failed to read Wayland events: {:?}", err));
                        },
                    }
                    event_queue
                        .dispatch_pending(state)
                        .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;
                    break;
                },
                None => {
                    event_queue
                        .dispatch_pending(state)
                        .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;
                    continue;
                },
            }
        }

        Ok(())
    }

    pub fn flush(&self) -> Result<(), String> {
        let mut data = self.loop_data.lock().unwrap();
        let (event_queue, _) = &mut *data;
        event_queue
            .flush()
            .map_err(|e| format!("Failed to flush Wayland queue: {:?}", e))?;
        Ok(())
    }

    fn initialize() -> Result<WaylandClient, String> {
        log::debug!("Initializing Wayland client...");
        let connection =
            Connection::connect_to_env().map_err(|e| format!("Wayland connect error: {:?}", e))?;
        log::debug!("Connected to Wayland display");

        let (global_list, mut event_queue) = registry_queue_init::<WaylandClientState>(&connection)
            .map_err(|e| format!("Failed to init Wayland registry: {:?}", e))?;
        let queue_handle = event_queue.handle();
        log::debug!("Wayland registry initialized");

        let shared = Arc::new(SharedState {
            surfaces: Mutex::new(HashMap::new()),
            focused_surface_key: Mutex::new(None),
            #[cfg(feature = "global-menu")]
            appmenu_objects: Mutex::new(HashMap::new()),
            callback_to_surface: Mutex::new(HashMap::new()),
        });

        let mut state = WaylandClientState::new(shared.clone());

        // Perform an initial roundtrip so the compositor processes any pending requests.
        log::debug!("Wayland initialize: event_queue_ptr={:p}", &event_queue);
        event_queue
            .roundtrip(&mut state)
            .map_err(|e| format!("Initial Wayland roundtrip failed: {:?}", e))?;

        let globals = WaylandGlobals::bind_all(&global_list, &queue_handle)?;

        Ok(WaylandClient {
            connection,
            queue_handle,
            globals,
            shared,
            loop_data: Mutex::new((event_queue, state)),
        })
    }
}

impl SharedState {
    pub(crate) fn get_surface(&self, key: u32) -> Option<Arc<WaylandSurfaceInner>> {
        let mut map = self.surfaces.lock().unwrap();
        let surface = map.get(&key)?.upgrade();
        if surface.is_none() {
            log::trace!("Wayland get_surface: key={} weak ref expired", key);
            map.remove(&key);
        } else {
            log::trace!("Wayland get_surface: key={} found", key);
        }
        surface
    }

    pub(crate) fn get_focused_surface_key(&self) -> Option<u32> {
        *self.focused_surface_key.lock().unwrap()
    }

    pub(crate) fn set_focused_surface(&self, key: Option<u32>) {
        *self.focused_surface_key.lock().unwrap() = key;
    }

    pub(crate) fn surfaces(&self) -> &Mutex<HashMap<u32, Weak<WaylandSurfaceInner>>> {
        &self.surfaces
    }

    pub(crate) fn callback_to_surface(&self) -> &Mutex<HashMap<u32, u32>> {
        &self.callback_to_surface
    }
}

/// Type alias for the Wayland queue handle.
pub type WaylandQueueHandle = QueueHandle<WaylandClientState>;

