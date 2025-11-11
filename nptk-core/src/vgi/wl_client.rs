#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use wayland_client::backend::WaylandError;
use wayland_client::globals::{registry_queue_init, GlobalList};
use wayland_client::Proxy;
use wayland_client::protocol::{wl_callback, wl_compositor, wl_registry, wl_surface, wl_seat, wl_shm, wl_keyboard, wl_pointer, wl_region};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_protocols_plasma::server_decoration::client::{
    org_kde_kwin_server_decoration, org_kde_kwin_server_decoration_manager,
};

use super::wayland_surface::{InputEvent, PointerEvent, KeyboardEvent, WaylandSurfaceInner};

/// Singleton Wayland client used by all native Wayland surfaces.
pub(crate) struct WaylandClient {
    connection: Connection,
    queue_handle: QueueHandle<WaylandClientState>,
    globals: WaylandGlobals,
    shared: Arc<SharedState>,
    loop_data: Mutex<(EventQueue<WaylandClientState>, WaylandClientState)>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct WaylandGlobals {
    pub compositor: wl_compositor::WlCompositor,
    pub wm_base: xdg_wm_base::XdgWmBase,
    pub decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub kde_server_decoration_manager:
        Option<org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager>,
    pub shm: Option<wl_shm::WlShm>,
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
}

struct SharedState {
    surfaces: Mutex<HashMap<u32, Weak<WaylandSurfaceInner>>>,
    focused_surface_key: Mutex<Option<u32>>,
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

    pub fn dispatch_pending(&self) -> Result<(), String> {
        let mut data = self.loop_data.lock().unwrap();
        let (event_queue, state) = &mut *data;

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
                        Ok(_) => {}
                        Err(WaylandError::Io(ref err)) if err.kind() == ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(err) => {
                            return Err(format!("Failed to read Wayland events: {:?}", err));
                        }
                    }
                    event_queue
                        .dispatch_pending(state)
                        .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;
                    break;
                }
                None => {
                    event_queue
                        .dispatch_pending(state)
                        .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;
                    continue;
                }
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

    pub fn roundtrip(&self) -> Result<(), String> {
        let mut data = self.loop_data.lock().unwrap();
        let (event_queue, state) = &mut *data;
        event_queue
            .roundtrip(state)
            .map_err(|e| format!("Wayland roundtrip failed: {:?}", e))?;
        Ok(())
    }

    fn initialize() -> Result<WaylandClient, String> {
        let connection =
            Connection::connect_to_env().map_err(|e| format!("Wayland connect error: {:?}", e))?;

        let (global_list, mut event_queue) =
            registry_queue_init::<WaylandClientState>(&connection)
                .map_err(|e| format!("Failed to init Wayland registry: {:?}", e))?;
        let queue_handle = event_queue.handle();

        let shared = Arc::new(SharedState {
            surfaces: Mutex::new(HashMap::new()),
            focused_surface_key: Mutex::new(None),
        });

        let mut state = WaylandClientState {
            shared: shared.clone(),
        };

        // Perform an initial roundtrip so the compositor processes any pending requests.
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

const COMPOSITOR_VERSION: u32 = 4;
const XDG_WM_BASE_VERSION: u32 = 6;
const ZXDG_DECORATION_VERSION: u32 = 1;
const KDE_SERVER_DECORATION_VERSION: u32 = 1;
const WL_SHM_VERSION: u32 = 1;
const WL_SEAT_VERSION: u32 = 7;

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

        let decoration_manager = match globals.bind::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
            qh,
            1..=ZXDG_DECORATION_VERSION,
            (),
        ) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!("Failed to bind zxdg_decoration_manager_v1: {:?}", err));
            }
        };
        let kde_server_decoration_manager = match globals.bind::<
            org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager,
            _,
            _,
        >(qh, 1..=KDE_SERVER_DECORATION_VERSION, ()) {
            Ok(mgr) => Some(mgr),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => {
                return Err(format!(
                    "Failed to bind org_kde_kwin_server_decoration_manager: {:?}",
                    err
                ));
            }
        };

        let shm = match globals.bind::<wl_shm::WlShm, _, _>(qh, 1..=WL_SHM_VERSION, ()) {
            Ok(s) => Some(s),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind wl_shm: {:?}", err)),
        };

        let seat = match globals.bind::<wl_seat::WlSeat, _, _>(qh, 1..=WL_SEAT_VERSION, ()) {
            Ok(s) => Some(s),
            Err(wayland_client::globals::BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind wl_seat: {:?}", err)),
        };

        let mut pointer = None;
        let mut keyboard = None;
        if let Some(ref seat) = seat {
            pointer = Some(seat.get_pointer(qh, ()));
            keyboard = Some(seat.get_keyboard(qh, ()));
        }

        Ok(Self {
            compositor,
            wm_base,
            decoration_manager,
            kde_server_decoration_manager,
            shm,
            seat,
            pointer,
            keyboard,
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

impl Dispatch<wl_region::WlRegion, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _region: &wl_region::WlRegion,
        _event: wl_region::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_region has no events to handle.
    }
}

impl Dispatch<WlShmPool, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _pool: &WlShmPool,
        _event: wayland_client::protocol::wl_shm_pool::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events used.
    }
}

impl Dispatch<WlBuffer, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _buffer: &WlBuffer,
        _event: wayland_client::protocol::wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events used.
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _seat: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No-op
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { surface, serial: _, surface_x, surface_y, .. } => {
                let key = surface.id().protocol_id();
                *state.shared.focused_surface_key.lock().unwrap() = Some(key);
                if let Some(surface) = state.shared.get_surface(key) {
                    surface.push_input_event(InputEvent::Pointer(PointerEvent::Enter {
                        surface_x,
                        surface_y,
                    }));
                    surface.handle_frame_done();
                }
            }
            wl_pointer::Event::Leave { .. } => {
                let mut focused = state.shared.focused_surface_key.lock().unwrap();
                if let Some(key) = *focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Pointer(PointerEvent::Leave));
                        surface.handle_frame_done();
                    }
                }
                *focused = None;
            }
            wl_pointer::Event::Motion { time: _, surface_x, surface_y } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Pointer(PointerEvent::Motion {
                            surface_x,
                            surface_y,
                        }));
                        surface.handle_frame_done();
                    }
                }
            }
            wl_pointer::Event::Button { serial: _, time: _, button, state: button_state } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(button_state) = button_state.into_result() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::Button {
                                button,
                                state: button_state,
                            }));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_pointer::Event::Axis { time: _, axis, value } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(axis_kind) = axis.into_result() {
                            let event = match axis_kind {
                                wl_pointer::Axis::VerticalScroll => PointerEvent::Axis {
                                    horizontal: None,
                                    vertical: Some(value),
                                },
                                wl_pointer::Axis::HorizontalScroll => PointerEvent::Axis {
                                    horizontal: Some(value),
                                    vertical: None,
                                },
                                _ => PointerEvent::Axis {
                                    horizontal: None,
                                    vertical: None,
                                },
                            };
                            surface.push_input_event(InputEvent::Pointer(event));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_pointer::Event::AxisSource { axis_source } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(source) = axis_source.into_result() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::AxisSource {
                                source,
                            }));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_pointer::Event::AxisStop { time: _, axis } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if axis.into_result().is_ok() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::AxisStop));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_pointer::Event::AxisDiscrete { axis, discrete } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(axis_kind) = axis.into_result() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::AxisDiscrete {
                                axis: axis_kind,
                                discrete,
                            }));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_pointer::Event::AxisValue120 { axis, value120 } => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(axis_kind) = axis.into_result() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::AxisValue120 {
                                axis: axis_kind,
                                value120,
                            }));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_pointer::Event::Frame => {
                if let Some(key) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Pointer(PointerEvent::Frame));
                        surface.handle_frame_done();
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { surface, serial: _, keys: _ } => {
                let key = surface.id().protocol_id();
                *state.shared.focused_surface_key.lock().unwrap() = Some(key);
                if let Some(surface) = state.shared.get_surface(key) {
                    surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Enter));
                    surface.handle_frame_done();
                }
            }
            wl_keyboard::Event::Leave { serial: _, .. } => {
                let mut focused = state.shared.focused_surface_key.lock().unwrap();
                if let Some(key) = *focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Leave));
                        surface.handle_frame_done();
                    }
                }
                *focused = None;
            }
            wl_keyboard::Event::Key { serial: _, time: _, key, state: key_state } => {
                if let Some(key_surface) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key_surface) {
                        if let Ok(actual_state) = key_state.into_result() {
                            surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Key {
                                keycode: key,
                                state: actual_state,
                            }));
                            surface.handle_frame_done();
                        }
                    }
                }
            }
            wl_keyboard::Event::Modifiers {
                serial: _,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                if let Some(key_surface) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key_surface) {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Modifiers {
                            mods_depressed,
                            mods_latched,
                            mods_locked,
                            group,
                        }));
                        surface.handle_frame_done();
                    }
                }
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                if let Some(key_surface) = *state.shared.focused_surface_key.lock().unwrap() {
                    if let Some(surface) = state.shared.get_surface(key_surface) {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::RepeatInfo {
                            rate,
                            delay,
                        }));
                        surface.handle_frame_done();
                    }
                }
            }
            _ => {}
        }
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
        xdg_surf: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        surface_key: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                let sid = xdg_surf.id().protocol_id();
                log::debug!("Wayland: XdgSurface({}) Configure serial={} on xdg_surface#{}", surface_key, serial, sid);
                eprintln!("[NPTK/Wayland] ACK SEND serial={} on xdg_surface#{}", serial, sid);
                // Ack immediately on the SAME object instance that emitted the event
                xdg_surf.ack_configure(serial);
                eprintln!("[NPTK/Wayland] FLUSH (after ACK) for xdg_surface#{}", sid);
                let _ = WaylandClient::instance().flush();
                if let Some(surface) = state.shared.get_surface(*surface_key) {
                    surface.handle_configure_after_ack(serial);
                }
            }
            _ => {}
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
                    log::debug!("Wayland: XdgToplevel({}) Configure {}x{}", surface_key, width, height);
                    surface.handle_toplevel_configure(width, height);
                }
                xdg_toplevel::Event::Close => {
                    log::debug!("Wayland: XdgToplevel({}) Close", surface_key);
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
        if let wl_callback::Event::Done { .. } = event {
            log::trace!("Wayland: Frame done for surface {}", surface_key);
            if let Some(surface) = state.shared.get_surface(*surface_key) {
                surface.handle_frame_done();
            }
        }
    }
}

pub(crate) type WaylandQueueHandle = QueueHandle<WaylandClientState>;

impl Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _proxy: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _event: zxdg_decoration_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, u32> for WaylandClientState {
    fn event(
        state: &mut Self,
        _proxy: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        surface_key: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let Some(surface) = state.shared.get_surface(*surface_key) {
            match event {
                zxdg_toplevel_decoration_v1::Event::Configure { .. } => {
                    // Decoration mode/config changed; request a redraw so chrome can update.
                    surface.request_redraw();
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager,
        _event: org_kde_kwin_server_decoration_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration,
        _event: org_kde_kwin_server_decoration::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_shm::WlShm,
        _event: wl_shm::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

