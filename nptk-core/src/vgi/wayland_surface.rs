//! Native Wayland surface implementation using smithay-client-toolkit.
//!
//! This module provides a Wayland surface implementation that uses SCTK
//! for high-level Wayland abstractions and integrates with wgpu for rendering.

#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use vello::wgpu::{SurfaceTexture, TextureFormat};
use crate::vgi::surface::SurfaceTrait;

/// Native Wayland surface implementation using smithay-client-toolkit.
///
/// This struct provides a Wayland surface implementation that uses SCTK
/// for high-level Wayland abstractions and integrates with wgpu for rendering.
#[cfg(target_os = "linux")]
pub struct WaylandSurface {
    /// Wayland connection
    connection: wayland_client::Connection,
    /// Wayland event queue for processing events
    event_queue: wayland_client::EventQueue<WaylandState>,
    /// SCTK window
    window: smithay_client_toolkit::shell::xdg::window::Window,
    /// Wayland surface for committing
    wl_surface: wayland_client::protocol::wl_surface::WlSurface,
    /// wgpu surface created from Wayland window (using vello's wgpu types)
    /// TODO: This needs to be properly initialized once wgpu surface creation is implemented
    wgpu_surface: Option<vello::wgpu::Surface<'static>>,
    /// Current window size
    size: (u32, u32),
    /// Flag indicating if a redraw is needed
    needs_redraw: bool,
    /// Surface format
    format: TextureFormat,
    /// Pending texture (stored between get_current_texture and present)
    pending_texture: Option<SurfaceTexture>,
    /// Wayland state for event handling
    state: Arc<Mutex<WaylandState>>,
}

/// State for Wayland event handling
#[cfg(target_os = "linux")]
struct WaylandState {
    /// Current window size from configure events
    size: (u32, u32),
    /// Flag indicating if a redraw is needed
    needs_redraw: bool,
    /// Flag indicating if window should close
    should_close: bool,
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_client::protocol::wl_registry::WlRegistry, wayland_client::globals::GlobalListContents> for WaylandState {
    fn event(
        _state: &mut Self,
        _registry: &wayland_client::protocol::wl_registry::WlRegistry,
        event: wayland_client::protocol::wl_registry::Event,
        _data: &wayland_client::globals::GlobalListContents,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // Registry events are handled by GlobalListContents
        let _ = event;
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_client::protocol::wl_compositor::WlCompositor, smithay_client_toolkit::globals::GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _compositor: &wayland_client::protocol::wl_compositor::WlCompositor,
        _event: wayland_client::protocol::wl_compositor::Event,
        _data: &smithay_client_toolkit::globals::GlobalData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // Compositor events are handled elsewhere
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase, smithay_client_toolkit::globals::GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _wm_base: &wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase,
        _event: wayland_protocols::xdg::shell::client::xdg_wm_base::Event,
        _data: &smithay_client_toolkit::globals::GlobalData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // XdgWmBase events are handled elsewhere
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, smithay_client_toolkit::globals::GlobalData> for WaylandState {
    fn event(
        _state: &mut Self,
        _decoration_manager: &wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _event: wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::Event,
        _data: &smithay_client_toolkit::globals::GlobalData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // Decoration manager events are handled elsewhere
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_client::protocol::wl_surface::WlSurface, smithay_client_toolkit::compositor::SurfaceData> for WaylandState {
    fn event(
        _state: &mut Self,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _event: wayland_client::protocol::wl_surface::Event,
        _data: &smithay_client_toolkit::compositor::SurfaceData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // Surface events are handled elsewhere
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface, smithay_client_toolkit::shell::xdg::window::WindowData> for WaylandState {
    fn event(
        _state: &mut Self,
        _xdg_surface: &wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface,
        _event: wayland_protocols::xdg::shell::client::xdg_surface::Event,
        _data: &smithay_client_toolkit::shell::xdg::window::WindowData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // XdgSurface events are handled by XdgShell
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel, smithay_client_toolkit::shell::xdg::window::WindowData> for WaylandState {
    fn event(
        _state: &mut Self,
        _toplevel: &wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel,
        _event: wayland_protocols::xdg::shell::client::xdg_toplevel::Event,
        _data: &smithay_client_toolkit::shell::xdg::window::WindowData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // XdgToplevel events are handled by XdgShell
    }
}

#[cfg(target_os = "linux")]
impl wayland_client::Dispatch<wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, smithay_client_toolkit::shell::xdg::window::WindowData> for WaylandState {
    fn event(
        _state: &mut Self,
        _decoration: &wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        _event: wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Event,
        _data: &smithay_client_toolkit::shell::xdg::window::WindowData,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        // Decoration events are handled elsewhere
    }
}

#[cfg(target_os = "linux")]
impl smithay_client_toolkit::shell::xdg::window::WindowHandler for WaylandState {
    fn request_close(&mut self, _conn: &wayland_client::Connection, _qh: &wayland_client::QueueHandle<Self>, _window: &smithay_client_toolkit::shell::xdg::window::Window) {
        self.should_close = true;
    }

    fn configure(
        &mut self,
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
        _window: &smithay_client_toolkit::shell::xdg::window::Window,
        configure: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        _serial: u32,
    ) {
        if let (Some(w), Some(h)) = (configure.new_size.0, configure.new_size.1) {
            self.size = (w.get(), h.get());
            self.needs_redraw = true;
        }
    }
}

#[cfg(target_os = "linux")]
impl WaylandSurface {
    /// Create a new Wayland surface.
    ///
    /// # Arguments
    /// * `width` - Initial window width in pixels
    /// * `height` - Initial window height in pixels
    /// * `title` - Window title
    ///
    /// # Returns
    /// * `Ok(WaylandSurface)` if creation succeeded
    /// * `Err(String)` if creation failed
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, String> {
        use wayland_client::globals::registry_queue_init;
        use smithay_client_toolkit::compositor::{CompositorState, Surface};
        use smithay_client_toolkit::shell::xdg::window::WindowDecorations;
        use smithay_client_toolkit::shell::xdg::XdgShell;

        // Connect to Wayland display
        let connection = wayland_client::Connection::connect_to_env()
            .map_err(|e| format!("Failed to connect to Wayland display: {:?}", e))?;
        
        // Initialize registry and event queue
        let (globals, mut event_queue) = registry_queue_init(&connection)
            .map_err(|e| format!("Failed to initialize registry: {:?}", e))?;
        let qh = event_queue.handle();
        
        // Bind compositor
        let compositor_state = CompositorState::bind(&globals, &qh)
            .map_err(|e| format!("Failed to bind compositor: {:?}", e))?;
        
        // Bind xdg shell
        let xdg_shell = XdgShell::bind(&globals, &qh)
            .map_err(|e| format!("Failed to bind xdg shell: {:?}", e))?;
        
        // Create surface
        let wl_surface = compositor_state.create_surface(&qh);
        
        // Store wl_surface reference for later use (before creating window)
        // We'll need it for committing in present()
        let wl_surface_stored = wl_surface.clone();
        
        // Create window
        let window = xdg_shell.create_window(
            Surface::from(wl_surface),
            WindowDecorations::ServerDefault,
            &qh,
        );
        
        // Set window title
        window.set_title(title);
        
        // Set window size
        window.set_min_size(Some((width, height)));
        window.set_max_size(Some((width, height)));
        
        // Commit the surface to make the window visible
        wl_surface_stored.commit();
        
        // Roundtrip to ensure window is created
        let mut state = WaylandState {
            size: (width, height),
            needs_redraw: true,
            should_close: false,
        };
        event_queue.roundtrip(&mut state)
            .map_err(|e| format!("Failed to roundtrip: {:?}", e))?;
        
        // Roundtrip to ensure window is created
        
        // Create wgpu surface from raw window handle
        // TODO: vello::wgpu::Instance::create_surface API needs investigation
        // For now, we'll leave wgpu_surface as None and return an error
        // This can be refined once we understand the correct API
        let wgpu_surface = None;
        
        // Get surface format (default to Bgra8Unorm, will be updated when adapter is available)
        let format = TextureFormat::Bgra8Unorm;
        
        let state = Arc::new(Mutex::new(state));
        
        Ok(Self {
            connection,
            event_queue,
            window,
            wl_surface: wl_surface_stored,
            wgpu_surface,
            size: (width, height),
            needs_redraw: true,
            format,
            pending_texture: None,
            state,
        })
    }

    /// Get a reference to the event queue for external dispatching.
    pub fn event_queue(&mut self) -> &mut wayland_client::EventQueue<WaylandState> {
        &mut self.event_queue
    }
    
    /// Get a reference to the Wayland window.
    pub fn window(&self) -> &smithay_client_toolkit::shell::xdg::window::Window {
        &self.window
    }
}

#[cfg(target_os = "linux")]
impl SurfaceTrait for WaylandSurface {
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String> {
        // Dispatch events before getting texture
        self.dispatch_events()?;

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
        
        // Commit the surface to Wayland compositor
        self.wl_surface.commit();
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.size = (width, height);
        
        // Update shared state
        let mut state_guard = self.state.lock()
            .map_err(|e| format!("Failed to lock state: {:?}", e))?;
        state_guard.size = (width, height);
        state_guard.needs_redraw = true;
        
        // Resize the Wayland window
        self.window.set_min_size(Some((width, height)));
        self.window.set_max_size(Some((width, height)));
        
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
        // Dispatch pending events from the event queue
        // Update state from shared state
        let mut state_guard = self.state.lock()
            .map_err(|e| format!("Failed to lock state: {:?}", e))?;
        
        // Dispatch events (non-blocking)
        match self.event_queue.dispatch_pending(&mut *state_guard) {
            Ok(_) => {},
            Err(e) => {
                return Err(format!("Failed to dispatch events: {:?}", e));
            }
        }
        
        // Update local state from shared state
        self.size = state_guard.size;
        self.needs_redraw = state_guard.needs_redraw;
        state_guard.needs_redraw = false;
        
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

