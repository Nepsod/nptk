#![cfg(target_os = "linux")]
#![allow(missing_docs)]

//! Native Wayland surface implementation backed by a shared event loop.

use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use vello::wgpu::{self, SurfaceTexture};

use wayland_client::protocol::{wl_keyboard, wl_pointer, wl_surface};
use wayland_client::{Connection, Proxy};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols_plasma::server_decoration::client::org_kde_kwin_server_decoration;

use super::surface::SurfaceTrait;
use crate::vgi::wl_client::{WaylandClient, WaylandQueueHandle};

#[derive(Default)]
struct SurfaceState {
    size: (u32, u32),
    pending_size: Option<(u32, u32)>,
    needs_redraw: bool,
    configured: bool,
    should_close: bool,
    frame_callback: Option<wayland_client::protocol::wl_callback::WlCallback>,
    first_frame_seen: bool,
    input_events: Vec<InputEvent>,
}

#[derive(Debug, Clone)]
pub(crate) enum InputEvent {
    Pointer(PointerEvent),
    Keyboard(KeyboardEvent),
}

#[derive(Debug, Clone)]
pub(crate) enum PointerEvent {
    Enter { surface_x: f64, surface_y: f64 },
    Leave,
    Motion { surface_x: f64, surface_y: f64 },
    Button { button: u32, state: wl_pointer::ButtonState },
    Axis { horizontal: Option<f64>, vertical: Option<f64> },
    AxisSource {
        source: wl_pointer::AxisSource,
    },
    AxisStop,
    AxisDiscrete { axis: wl_pointer::Axis, discrete: i32 },
    AxisValue120 { axis: wl_pointer::Axis, value120: i32 },
    Frame,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum KeyboardEvent {
    Enter,
    Leave,
    Key { keycode: u32, state: wl_keyboard::KeyState },
    Modifiers {
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    },
    RepeatInfo { rate: i32, delay: i32 },
}

pub(crate) struct WaylandSurfaceInner {
    surface_key: u32,
    wl_surface: wl_surface::WlSurface,
    #[allow(dead_code)]
    xdg_surface: xdg_surface::XdgSurface,
    #[allow(dead_code)]
    xdg_toplevel: xdg_toplevel::XdgToplevel,
    #[allow(dead_code)]
    _decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    #[allow(dead_code)]
    _kde_decoration: Option<org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration>,
    queue_handle: WaylandQueueHandle,
    state: Mutex<SurfaceState>,
}

impl WaylandSurfaceInner {
    fn new(
        wl_surface: wl_surface::WlSurface,
        xdg_surface: xdg_surface::XdgSurface,
        xdg_toplevel: xdg_toplevel::XdgToplevel,
        queue_handle: WaylandQueueHandle,
        initial_size: (u32, u32),
    ) -> Arc<Self> {
        let surface_key = wl_surface.id().protocol_id();
        let mut state = SurfaceState::default();
        state.size = initial_size;

        Arc::new(Self {
            surface_key,
            wl_surface,
            xdg_surface,
            xdg_toplevel,
            _decoration: None,
            _kde_decoration: None,
            queue_handle,
            state: Mutex::new(state),
        })
    }

    pub(crate) fn push_input_event(&self, event: InputEvent) {
        let mut state = self.state.lock().unwrap();
        state.input_events.push(event);
    }

    fn take_input_events(&self) -> Vec<InputEvent> {
        let mut state = self.state.lock().unwrap();
        if state.input_events.is_empty() {
            Vec::new()
        } else {
            let mut events = Vec::with_capacity(state.input_events.len());
            events.append(&mut state.input_events);
            events
        }
    }

    pub(crate) fn surface_key(&self) -> u32 {
        self.surface_key
    }

    pub(crate) fn handle_toplevel_configure(&self, width: i32, height: i32) {
        log::debug!("Wayland toplevel configure: {}x{}", width, height);
        if width <= 0 || height <= 0 {
            return;
        }
        let mut state = self.state.lock().unwrap();
        state.pending_size = Some((width as u32, height as u32));
    }

    pub(crate) fn handle_configure(&self, serial: u32) {
        log::debug!("Wayland xdg_surface configure: serial={}", serial);
        let mut state = self.state.lock().unwrap();

        let size = state
            .pending_size
            .take()
            .unwrap_or_else(|| state.size);

        let width = size.0.max(1);
        let height = size.1.max(1);

        self.xdg_surface
            .set_window_geometry(0, 0, width as i32, height as i32);
        self.xdg_surface.ack_configure(serial);
        // Ensure the compositor sees the ACK immediately
        let _ = WaylandClient::instance().flush();

        state.size = (width, height);
        state.configured = true;
        state.needs_redraw = true;
        log::debug!(
            "Wayland configure applied: size={}x{}, set configured=true, needs_redraw=true",
            width,
            height
        );
        eprintln!(
            "[NPTK/Wayland] configured: size={}x{}, configured=true, needs_redraw=true",
            width, height
        );

        self.ensure_frame_callback_locked(&mut state);
    }

    pub(crate) fn handle_frame_done(&self) {
        log::trace!("Wayland frame callback done");
        let mut state = self.state.lock().unwrap();
        state.frame_callback = None;
        state.needs_redraw = true;
        state.first_frame_seen = true;
    }

    pub(crate) fn mark_closed(&self) {
        let mut state = self.state.lock().unwrap();
        state.should_close = true;
    }

    fn after_present(&self) {
        let mut state = self.state.lock().unwrap();
        state.needs_redraw = false;
        self.ensure_frame_callback_locked(&mut state);
    }

    fn ensure_frame_callback_locked(&self, state: &mut SurfaceState) {
        if state.frame_callback.is_none() {
            let callback = self
                .wl_surface
                .frame(&self.queue_handle, self.surface_key);
            state.frame_callback = Some(callback);
            let _ = WaylandClient::instance().flush();
            log::trace!("Registered wl_surface.frame callback");
        }
    }

    pub(crate) fn request_redraw(&self) {
        let mut state = self.state.lock().unwrap();
        state.needs_redraw = true;
        self.ensure_frame_callback_locked(&mut state);
    }

    fn take_status(&self) -> SurfaceStatus {
        let mut state = self.state.lock().unwrap();
        let status = SurfaceStatus {
            size: state.size,
            needs_redraw: state.needs_redraw,
            configured: state.configured,
            should_close: state.should_close,
        };
        state.needs_redraw = false;
        state.configured = false;
        status
    }

    fn wl_surface(&self) -> wl_surface::WlSurface {
        self.wl_surface.clone()
    }
}

struct SurfaceStatus {
    size: (u32, u32),
    needs_redraw: bool,
    configured: bool,
    should_close: bool,
}

/// Manual Wayland surface implementation backed by the shared client.
pub struct WaylandSurface {
    client: Arc<WaylandClient>,
    inner: Arc<WaylandSurfaceInner>,
    pub(crate) wgpu_surface: Option<wgpu::Surface<'static>>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    is_configured: bool,
    needs_redraw: bool,
    pending_reconfigure: bool,
    pending_input_events: Vec<InputEvent>,
}

impl WaylandSurface {
    pub fn new(
        width: u32,
        height: u32,
        title: &str,
        gpu_context: &crate::vgi::GpuContext,
    ) -> Result<Self, String> {
        let client = WaylandClient::instance();
        let globals = client.globals();
        let queue_handle = client.queue_handle();

        let wl_surface: wl_surface::WlSurface = globals.compositor.create_surface(&queue_handle, ());
        let surface_key = wl_surface.id().protocol_id();
        let xdg_surface = globals
            .wm_base
            .get_xdg_surface(&wl_surface, &queue_handle, surface_key);
        let xdg_toplevel =
            xdg_surface.get_toplevel(&queue_handle, surface_key);

        xdg_toplevel.set_title(title.to_owned());
        xdg_toplevel.set_app_id("nptk".to_owned());

        let mut inner = WaylandSurfaceInner::new(
            wl_surface.clone(),
            xdg_surface.clone(),
            xdg_toplevel.clone(),
            queue_handle.clone(),
            (width.max(1), height.max(1)),
        );
        // Request server-side decorations if available
        // Prefer KDE server decorations on KWin if available (create decoration over wl_surface)
        if let Some(kde_dm) = client.globals().kde_server_decoration_manager.clone() {
            // Create a KDE server decoration object tied to this wl_surface
            let kde_deco = kde_dm.create(&wl_surface, &queue_handle, ());
            if let Some(inner_mut) = Arc::get_mut(&mut inner) {
                inner_mut._kde_decoration = Some(kde_deco);
            }
        }
        if let Some(dm) = client.globals().decoration_manager {
            let deco = dm.get_toplevel_decoration(&inner.xdg_toplevel, &queue_handle, surface_key);
            deco.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
            // store to keep alive
            let inner_mut = Arc::get_mut(&mut inner).expect("WaylandSurfaceInner not shared yet");
            inner_mut._decoration = Some(deco);
            // Commit decoration state so compositor sends updated configure
            wl_surface.commit();
            let _ = client.flush();
        }
        client.register_surface(&inner);
        
        // Commit the surface after registering so we can handle configure events
        wl_surface.commit();
        // Flush immediately so the compositor sees the commit
        client.flush()?;
        
        // Request an initial frame; compositor will trigger redraw via callback
        {
            let mut s = inner.state.lock().unwrap();
            s.needs_redraw = true;
            inner.ensure_frame_callback_locked(&mut s);
        }
        // Process pending events to pick up the initial configure as early as possible
        let _ = client.dispatch_pending();

        let connection = client.connection();
        let (wgpu_surface, format) =
            Self::create_wgpu_surface(&connection, &wl_surface, gpu_context)?;

        Ok(Self {
            client,
            inner,
            wgpu_surface,
            format,
            size: (width.max(1), height.max(1)),
            is_configured: false,
            needs_redraw: false,
            pending_reconfigure: true,
            pending_input_events: Vec::new(),
        })
    }

    fn create_wgpu_surface(
        connection: &Connection,
        wl_surface: &wl_surface::WlSurface,
        gpu_context: &crate::vgi::GpuContext,
    ) -> Result<(Option<wgpu::Surface<'static>>, wgpu::TextureFormat), String> {
        let surface_ptr =
            NonNull::new(wl_surface.id().as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| "Invalid Wayland surface pointer".to_string())?;
        let display_ptr =
            NonNull::new(connection.display().id().as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| "Invalid Wayland display pointer".to_string())?;

        let raw_window = WaylandWindowHandle::new(surface_ptr);
        let raw_display = WaylandDisplayHandle::new(display_ptr);

        let target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: RawDisplayHandle::Wayland(raw_display),
            raw_window_handle: RawWindowHandle::Wayland(raw_window),
        };

        match unsafe { gpu_context.instance().create_surface_unsafe(target) } {
            Ok(surface) => Ok((Some(surface), wgpu::TextureFormat::Bgra8Unorm)),
            Err(e) => Err(format!("Failed to create Wayland wgpu surface: {:?}", e)),
        }
    }
}

impl Drop for WaylandSurface {
    fn drop(&mut self) {
        self.client
            .unregister_surface(self.inner.surface_key());
    }
}

impl SurfaceTrait for WaylandSurface {
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String> {
        if !self.is_configured {
            return Err("Wayland surface not configured yet".to_string());
        }

        let wgpu_surface = self
            .wgpu_surface
            .as_ref()
            .ok_or_else(|| "wgpu surface is not initialised".to_string())?;

        wgpu_surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get Wayland surface texture: {:?}", e))
    }

    fn present(&mut self) -> Result<(), String> {
        log::debug!("Wayland present(): committing wl_surface");
        self.inner.after_present();
        self.inner.wl_surface().commit();
        self.needs_redraw = false;
        // Flush immediately after commit so the compositor sees the new buffer
        self.client.flush()?;
        // Nudge delivery of the first frame callback to avoid startup stalls.
        if !self.first_frame_seen() {
            let _ = self.client.roundtrip();
        }
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        let mut state = self.inner.state.lock().unwrap();
        state.pending_size = Some((width.max(1), height.max(1)));
        state.needs_redraw = true;
        self.inner.ensure_frame_callback_locked(&mut state);
        Ok(())
    }

    fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn needs_event_dispatch(&self) -> bool {
        true
    }

    fn dispatch_events(&mut self) -> Result<bool, String> {
        // Drive Wayland event processing on the owning thread.
        self.client.dispatch_pending()?;

        let status = self.inner.take_status();
        if self.size != status.size {
            log::debug!("Wayland dispatch: size changed to {}x{}", status.size.0, status.size.1);
        }
        self.size = status.size;
        if status.configured {
            self.is_configured = true;
            self.pending_reconfigure = true;
            log::debug!("Wayland dispatch: configured=true, pending_reconfigure=true");
            eprintln!("[NPTK/Wayland] dispatch: configured=true, pending_reconfigure=true");
        }
        if status.needs_redraw {
            self.needs_redraw = true;
            log::trace!("Wayland dispatch: needs_redraw=true from status");
            eprintln!("[NPTK/Wayland] dispatch: needs_redraw=true");
        }

        // If we just got configured and require reconfiguration, request a redraw immediately
        // so the higher layers render once and present a buffer to get mapped.
        if self.is_configured && self.pending_reconfigure {
            self.needs_redraw = true;
            log::debug!("Wayland dispatch: forcing redraw after configure");
            eprintln!("[NPTK/Wayland] dispatch: forcing redraw after configure");
        }

        if status.should_close {
            self.needs_redraw = false;
            return Err("Wayland surface requested close".to_string());
        }

        let mut new_events = self.inner.take_input_events();
        if !new_events.is_empty() {
            self.pending_input_events.append(&mut new_events);
        }

        Ok(self.needs_redraw)
    }
}

impl WaylandSurface {
    pub fn configure_surface(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        present_mode: wgpu::PresentMode,
    ) -> Result<(), String> {
        let wgpu_surface = self
            .wgpu_surface
            .as_ref()
            .ok_or_else(|| "wgpu surface not initialised".to_string())?;

        // Use the size from the inner state, which should be updated after the configure event
        let state = self.inner.state.lock().unwrap();
        let size = state.size;
        drop(state);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0,
            height: size.1,
            present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        wgpu_surface.configure(device, &config);
        self.format = format;
        self.size = size;
        self.is_configured = true;
        self.needs_redraw = true;
        self.pending_reconfigure = false;
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    pub fn first_frame_seen(&self) -> bool {
        let state = self.inner.state.lock().unwrap();
        state.first_frame_seen
    }

    pub fn should_close(&self) -> bool {
        let state = self.inner.state.lock().unwrap();
        state.should_close
    }

    pub fn requires_reconfigure(&self) -> bool {
        self.pending_reconfigure
    }

    pub(crate) fn take_pending_input_events(&mut self) -> Vec<InputEvent> {
        if self.pending_input_events.is_empty() {
            Vec::new()
        } else {
            self.pending_input_events.drain(..).collect()
        }
    }
}

