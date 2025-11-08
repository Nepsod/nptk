//! Native Wayland surface implementation using manual XDG management.
//!
//! This module mirrors the GPUI Wayland pipeline: we create and manage XDG surfaces
//! directly, acknowledge configure events ourselves, and only render when the compositor
//! requests a new frame.

use crate::vgi::surface::SurfaceTrait;
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use vello::wgpu::{SurfaceTexture, TextureFormat};

#[cfg(target_os = "linux")]
use wayland_client::{
    globals::{registry_queue_init, BindError, GlobalList},
    protocol::{wl_callback, wl_compositor, wl_registry, wl_surface},
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
#[cfg(target_os = "linux")]
use wayland_protocols::xdg::{
    decoration::zv1::client::zxdg_decoration_manager_v1,
    shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};

#[cfg(target_os = "linux")]
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
#[cfg(target_os = "linux")]
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};

#[cfg(target_os = "linux")]
const COMPOSITOR_VERSION: u32 = 4;
#[cfg(target_os = "linux")]
const XDG_WM_BASE_VERSION: u32 = 4;
#[cfg(target_os = "linux")]
const DECORATION_MANAGER_VERSION: u32 = 1;

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct WaylandGlobals {
    compositor: wl_compositor::WlCompositor,
    wm_base: xdg_wm_base::XdgWmBase,
    decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
}

#[cfg(target_os = "linux")]
impl WaylandGlobals {
    fn bind_all(
        globals: &GlobalList,
        qh: &QueueHandle<WaylandState>,
    ) -> Result<Self, String> {
        let compositor = globals
            .bind::<wl_compositor::WlCompositor, _, _>(qh, 1..=COMPOSITOR_VERSION, ())
            .map_err(|e| format!("Failed to bind wl_compositor: {:?}", e))?;

        let wm_base = globals
            .bind::<xdg_wm_base::XdgWmBase, _, _>(qh, 1..=XDG_WM_BASE_VERSION, ())
            .map_err(|e| format!("Failed to bind xdg_wm_base: {:?}", e))?;

        let decoration_manager = match globals.bind::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
            qh,
            1..=DECORATION_MANAGER_VERSION,
            (),
        ) {
            Ok(proxy) => Some(proxy),
            Err(BindError::NotPresent) => None,
            Err(err) => return Err(format!("Failed to bind decoration manager: {:?}", err)),
        };

        Ok(Self {
            compositor,
            wm_base,
            decoration_manager,
        })
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct PendingConfigure {
    serial: u32,
    new_size: Option<(u32, u32)>,
}

/// Wayland state shared with the event queue.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(crate) struct WaylandState {
    pending_size: Option<(u32, u32)>,
    pending_configure: Option<PendingConfigure>,
    current_size: (u32, u32),
    needs_redraw: bool,
    should_close: bool,
}

#[cfg(target_os = "linux")]
impl WaylandState {
    fn take_pending_configure(&mut self) -> Option<PendingConfigure> {
        self.pending_configure.take()
    }
}

#[cfg(target_os = "linux")]
impl Default for WaylandState {
    fn default() -> Self {
        Self {
            pending_size: None,
            pending_configure: None,
            current_size: (1, 1),
            needs_redraw: false,
            should_close: false,
        }
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<wl_registry::WlRegistry, GlobalList> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalList,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // GlobalList handles registry bookkeeping for us.
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &wayland_client::globals::GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_compositor::WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We don't need compositor-specific events.
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        proxy: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()> for WaylandState {
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

#[cfg(target_os = "linux")]
impl Dispatch<wl_surface::WlSurface, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<xdg_surface::XdgSurface, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            let new_size = state.pending_size.take();
            state.pending_configure = Some(PendingConfigure { serial, new_size });
            state.needs_redraw = true;
        }
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                if width > 0 && height > 0 {
                    state.pending_size = Some((width as u32, height as u32));
                } else {
                    state.pending_size = None;
                }
            }
            xdg_toplevel::Event::Close => {
                state.should_close = true;
            }
            _ => {}
        }
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        _event: zxdg_toplevel_decoration_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(target_os = "linux")]
impl Dispatch<wl_callback::WlCallback, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.needs_redraw = true;
        }
    }
}

/// Manual Wayland surface implementation.
#[cfg(target_os = "linux")]
pub struct WaylandSurface {
    _connection: Connection,
    event_queue: EventQueue<WaylandState>,
    _globals: WaylandGlobals,
    wl_surface: wl_surface::WlSurface,
    xdg_surface: xdg_surface::XdgSurface,
    xdg_toplevel: xdg_toplevel::XdgToplevel,
    _decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    pub(crate) wgpu_surface: Option<vello::wgpu::Surface<'static>>,
    is_configured: bool,
    size: (u32, u32),
    needs_redraw: bool,
    acknowledged_first_configure: bool,
    format: TextureFormat,
    pending_texture: Option<SurfaceTexture>,
    state: Arc<Mutex<WaylandState>>,
    frame_callback: Option<wl_callback::WlCallback>,
}

#[cfg(target_os = "linux")]
impl WaylandSurface {
    pub fn new(width: u32, height: u32, title: &str, gpu_context: &crate::vgi::GpuContext) -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|e| format!("Failed to connect to Wayland display: {:?}", e))?;

        let (global_list, mut event_queue) =
            registry_queue_init::<WaylandState>(&connection)
                .map_err(|e| format!("Failed to initialize registry: {:?}", e))?;
        let qh = event_queue.handle();

        let globals = WaylandGlobals::bind_all(&global_list, &qh)
            .map_err(|e| format!("Failed to bind Wayland globals: {}", e))?;

        let wl_surface: wl_surface::WlSurface = globals.compositor.create_surface(&qh, ());
        let xdg_surface = globals.wm_base.get_xdg_surface(&wl_surface, &qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());
        let decoration = globals
            .decoration_manager
            .as_ref()
            .map(|manager| manager.get_toplevel_decoration(&xdg_toplevel, &qh, ()));

        xdg_toplevel.set_title(title.to_owned());
        if let Ok(app_id) = std::env::var("NPTK_APP_ID") {
            xdg_toplevel.set_app_id(app_id);
        }
        xdg_toplevel.set_min_size(width as i32, height as i32);
        xdg_toplevel.set_max_size(width as i32, height as i32);

        wl_surface.commit();

        let mut state = WaylandState {
            pending_size: Some((width, height)),
            pending_configure: None,
            current_size: (width, height),
            needs_redraw: true,
            should_close: false,
        };
        event_queue
            .roundtrip(&mut state)
            .map_err(|e| format!("Wayland roundtrip failed: {:?}", e))?;

        let (wgpu_surface, format) =
            Self::create_wgpu_surface(&connection, &wl_surface, gpu_context)?;

        let acknowledged_first_configure = state.pending_configure.is_some();
        let state_arc = Arc::new(Mutex::new(state));

        Ok(Self {
            _connection: connection,
            event_queue,
            _globals: globals,
            wl_surface,
            xdg_surface: xdg_surface.into(),
            xdg_toplevel: xdg_toplevel.into(),
            _decoration: decoration.map(|d| d.into()),
            wgpu_surface,
            is_configured: false,
            size: (width, height),
            needs_redraw: true,
            acknowledged_first_configure,
            format,
            pending_texture: None,
            state: state_arc,
            frame_callback: None,
        })
    }

    fn create_wgpu_surface(
        connection: &Connection,
        wl_surface: &wl_surface::WlSurface,
        gpu_context: &crate::vgi::GpuContext,
    ) -> Result<(Option<vello::wgpu::Surface<'static>>, TextureFormat), String> {
        use std::ptr::NonNull;
        let surface_ptr =
            NonNull::new(wl_surface.id().as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| "Invalid Wayland surface pointer".to_string())?;
        let display_ptr =
            NonNull::new(connection.display().id().as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| "Invalid Wayland display pointer".to_string())?;

        let raw_window = WaylandWindowHandle::new(surface_ptr);
        let raw_display = WaylandDisplayHandle::new(display_ptr);

        let target = vello::wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: RawDisplayHandle::Wayland(raw_display),
            raw_window_handle: RawWindowHandle::Wayland(raw_window),
        };

        match unsafe { gpu_context.instance().create_surface_unsafe(target) } {
            Ok(surface) => Ok((Some(surface), TextureFormat::Bgra8Unorm)),
            Err(e) => Err(format!("Failed to create Wayland wgpu surface: {:?}", e)),
        }
    }

    fn request_frame_callback(&mut self) {
        let qh = self.event_queue.handle();
        let _ = self.frame_callback.take();
        let callback = self.wl_surface.frame(&qh, ());
        self.frame_callback = Some(callback.into());
    }

    fn handle_pending_configure(&mut self, pending: PendingConfigure) {
        if let Some((w, h)) = pending.new_size {
            self.size = (w.max(1), h.max(1));
            self.xdg_surface
                .set_window_geometry(0, 0, self.size.0 as i32, self.size.1 as i32);
        }
        self.xdg_surface.ack_configure(pending.serial);

        if let Ok(mut state) = self.state.lock() {
            state.current_size = self.size;
        }

        if !self.acknowledged_first_configure {
            self.acknowledged_first_configure = true;
            self.request_frame_callback();
        }

        self.needs_redraw = true;
    }

    /// Configure the wgpu surface for rendering.
    ///
    /// This must be called before `get_current_texture()` can be used.
    /// The surface will be reconfigured if the size changes.
    ///
    /// # Arguments
    /// * `device` - The GPU device
    /// * `format` - The surface format
    /// * `present_mode` - The presentation mode
    pub fn configure_surface(
        &mut self,
        device: &vello::wgpu::Device,
        format: TextureFormat,
        present_mode: vello::wgpu::PresentMode,
    ) -> Result<(), String> {
        let wgpu_surface = self.wgpu_surface.as_mut()
            .ok_or_else(|| "wgpu surface not initialized".to_string())?;
        
        let config = vello::wgpu::SurfaceConfiguration {
            usage: vello::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: self.size.0,
            height: self.size.1,
            present_mode,
            alpha_mode: vello::wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        wgpu_surface.configure(device, &config);
        self.is_configured = true;
        self.format = format;
        
        log::debug!("Configured Wayland wgpu surface: {}x{} format={:?} present_mode={:?}", 
            self.size.0, self.size.1, format, present_mode);
        
        Ok(())
    }
    
    /// Check if the surface is configured.
    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    /// Check if the window should close.
    pub fn should_close(&self) -> bool {
        self.state.lock()
            .map(|s| s.should_close)
            .unwrap_or(false)
    }
}

#[cfg(target_os = "linux")]
impl SurfaceTrait for WaylandSurface {
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String> {
        // Dispatch events before getting texture
        self.dispatch_events()?;

        // Ensure surface is configured
        if !self.is_configured {
            return Err("Wayland wgpu surface is not configured. Call configure_surface() first.".to_string());
        }

        // Get current texture from wgpu surface
        let wgpu_surface = self.wgpu_surface.as_ref()
            .ok_or_else(|| "Wayland wgpu surface not initialized. wgpu surface creation needs to be implemented.".to_string())?;
        
        let texture = wgpu_surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {:?}", e))?;
        
        // Store pending texture for present()
        // Note: We store it here, but the caller will also have a reference to it
        // The texture will be presented when present() is called
        self.pending_texture = Some(texture);
        
        // Return the stored texture
        // SAFETY: We just stored it, so we can unwrap safely
        // The texture will be moved out here, but we'll handle present() differently
        Ok(self.pending_texture.take().unwrap())
    }

    fn present(&mut self) -> Result<(), String> {
        // Present the pending texture if available
        // Note: The texture was already taken in get_current_texture(), so we need to handle this differently
        // For Wayland, we'll commit the surface after rendering is complete
        // The texture presentation is handled by wgpu when the texture is dropped (RAII)
        
        // CRITICAL: For Wayland, wgpu automatically attaches a buffer when rendering.
        // We need to commit the surface AFTER the buffer is attached to make the window visible.
        // This is what GPUI does in completed_frame() (line 1221).
        
        // Request frame callback for next frame (for smooth rendering)
        // This ensures we only render when the compositor is ready
        let qh = self.event_queue.handle();
        // Drop old callback if it exists (automatically destroyed)
        let _ = self.frame_callback.take();
        // Request new frame callback with () as userdata
        let callback = self.wl_surface.frame(&qh, ());
        self.frame_callback = Some(callback);
        
        // Commit the surface to Wayland compositor
        // This makes the rendered frame visible
        // wgpu has already attached a buffer during rendering, so this commit will show it
        log::debug!("Committing Wayland surface after rendering (buffer should be attached by wgpu)");
        self.wl_surface.commit();
        
        self.needs_redraw = false;
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.size = (width, height);
        if let Ok(mut state_guard) = self.state.lock() {
            state_guard.current_size = self.size;
            state_guard.pending_size = Some(self.size);
            state_guard.needs_redraw = true;
        }

        self.xdg_surface
            .set_window_geometry(0, 0, width as i32, height as i32);
        self.xdg_toplevel.set_min_size(width as i32, height as i32);
        self.xdg_toplevel.set_max_size(width as i32, height as i32);

        if self.is_configured {
            self.is_configured = false;
            log::debug!("Surface resized to {}x{}, needs reconfiguration", width, height);
        }

        self.needs_redraw = true;
        Ok(())
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn needs_event_dispatch(&self) -> bool {
        true
    }

    fn dispatch_events(&mut self) -> Result<bool, String> {
        let mut state_guard = self
            .state
            .lock()
            .map_err(|e| format!("Failed to lock Wayland state: {:?}", e))?;

        self.event_queue
            .dispatch_pending(&mut *state_guard)
            .map_err(|e| format!("Failed to dispatch Wayland events: {:?}", e))?;

        let pending = state_guard.take_pending_configure();
        let should_close = state_guard.should_close;
        let needs_redraw_flag = state_guard.needs_redraw;
        let current_size = state_guard.current_size;
        state_guard.needs_redraw = false;
        drop(state_guard);

        if let Some(pending) = pending {
            self.handle_pending_configure(pending);
        } else {
            self.size = current_size;
        }

        if should_close {
            log::debug!("Wayland window close requested");
        }

        if needs_redraw_flag {
            self.needs_redraw = true;
        }

        Ok(self.needs_redraw)
    }
}

#[cfg(not(target_os = "linux"))]
pub struct WaylandSurface;

#[cfg(not(target_os = "linux"))]
impl WaylandSurface {
    pub fn new(_width: u32, _height: u32, _title: &str) -> Result<Self, String> {
        Err("Wayland surfaces are only available on Linux".to_string())
    }
}

