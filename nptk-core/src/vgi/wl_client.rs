#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::thread;

use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use wayland_client::globals::{registry_queue_init, GlobalList};
use wayland_client::protocol::{wl_callback, wl_compositor, wl_registry, wl_surface};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use super::wayland_surface::WaylandSurfaceInner;

/// Singleton Wayland client used by all native Wayland surfaces.
pub(crate) struct WaylandClient {
    connection: Connection,
    queue_handle: QueueHandle<WaylandClientState>,
    globals: WaylandGlobals,
    shared: Arc<SharedState>,
    _event_loop_thread: thread::JoinHandle<()>,
}

#[derive(Clone)]
pub(crate) struct WaylandGlobals {
    pub compositor: wl_compositor::WlCompositor,
    pub wm_base: xdg_wm_base::XdgWmBase,
}

struct SharedState {
    surfaces: Mutex<HashMap<u32, Weak<WaylandSurfaceInner>>>,
}

pub(crate) struct WaylandClientState {
    shared: Arc<SharedState>,
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
        map.insert(surface.surface_key(), Arc::downgrade(surface));
    }

    pub fn unregister_surface(&self, surface_key: u32) {
        let mut map = self.shared.surfaces.lock().unwrap();
        map.remove(&surface_key);
    }

    fn initialize() -> Result<WaylandClient, String> {
        let connection =
            Connection::connect_to_env().map_err(|e| format!("Wayland connect error: {:?}", e))?;

        let (global_list, event_queue) =
            registry_queue_init::<WaylandClientState>(&connection)
                .map_err(|e| format!("Failed to init Wayland registry: {:?}", e))?;
        let queue_handle = event_queue.handle();

        let globals = WaylandGlobals::bind_all(&global_list, &queue_handle)?;

        let shared = Arc::new(SharedState {
            surfaces: Mutex::new(HashMap::new()),
        });

        let shared_state = shared.clone();
        let connection_thread = connection.clone();
        let event_loop_thread = thread::Builder::new()
            .name("nptk-wayland".into())
            .spawn(move || {
                let mut event_loop = EventLoop::<WaylandClientState>::try_new()
                    .expect("Failed to create Wayland event loop");
                WaylandSource::new(connection_thread, event_queue)
                    .insert(event_loop.handle())
                    .expect("Failed to insert Wayland source");

                let mut state = WaylandClientState {
                    shared: shared_state,
                };

                event_loop
                    .run(None, &mut state, |_| {})
                    .expect("Wayland event loop failed");
            })
            .map_err(|e| format!("Failed to spawn Wayland loop: {:?}", e))?;

        Ok(WaylandClient {
            connection,
            queue_handle,
            globals,
            shared,
            _event_loop_thread: event_loop_thread,
        })
    }
}

const COMPOSITOR_VERSION: u32 = 4;
const XDG_WM_BASE_VERSION: u32 = 6;

impl WaylandGlobals {
    fn bind_all(
        globals: &GlobalList,
        qh: &QueueHandle<WaylandClientState>,
    ) -> Result<Self, String> {
        let compositor: wl_compositor::WlCompositor =
            globals
                .bind(qh, 1..=COMPOSITOR_VERSION, ())
                .map_err(|e| format!("Failed to bind wl_compositor: {:?}", e))?;

        let wm_base: xdg_wm_base::XdgWmBase =
            globals
                .bind(qh, 1..=XDG_WM_BASE_VERSION, ())
                .map_err(|e| format!("Failed to bind xdg_wm_base: {:?}", e))?;

        Ok(Self {
            compositor,
            wm_base,
        })
    }
}

impl SharedState {
    fn get_surface(&self, key: u32) -> Option<Arc<WaylandSurfaceInner>> {
        let mut map = self.surfaces.lock().unwrap();
        let surface = map.get(&key)?.upgrade();
        if surface.is_none() {
            map.remove(&key);
        }
        surface
    }
}

impl Dispatch<wl_registry::WlRegistry, wayland_client::globals::GlobalListContents>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &wayland_client::globals::GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Currently unused.
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _compositor: &wl_compositor::WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _surface: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We do not currently react to wl_surface events.
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, u32> for WaylandClientState {
    fn event(
        state: &mut Self,
        _xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        surface_key: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            if let Some(surface) = state.shared.get_surface(*surface_key) {
                surface.handle_configure(serial);
            }
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, u32> for WaylandClientState {
    fn event(
        state: &mut Self,
        _toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        surface_key: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let Some(surface) = state.shared.get_surface(*surface_key) {
            match event {
                xdg_toplevel::Event::Configure { width, height, .. } => {
                    surface.handle_toplevel_configure(width, height);
                }
                xdg_toplevel::Event::Close => {
                    surface.mark_closed();
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_callback::WlCallback, u32> for WaylandClientState {
    fn event(
        state: &mut Self,
        _callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        surface_key: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_callback::Event::Done { .. }) {
            if let Some(surface) = state.shared.get_surface(*surface_key) {
                surface.handle_frame_done();
            }
        }
    }
}

pub(crate) type WaylandQueueHandle = QueueHandle<WaylandClientState>;

