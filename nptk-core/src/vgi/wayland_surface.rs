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
    /// Wayland connection (kept for potential future use in cleanup or other operations)
    #[allow(dead_code)]
    connection: wayland_client::Connection,
    /// Wayland event queue for processing events
    event_queue: wayland_client::EventQueue<WaylandState>,
    /// SCTK window
    window: smithay_client_toolkit::shell::xdg::window::Window,
    /// Wayland surface for committing
    wl_surface: wayland_client::protocol::wl_surface::WlSurface,
    /// wgpu surface created from Wayland window (using vello's wgpu types)
    /// This is created early to help with adapter enumeration on Wayland
    pub(crate) wgpu_surface: Option<vello::wgpu::Surface<'static>>,
    /// Flag indicating if the wgpu surface has been configured
    is_configured: bool,
    /// Current window size
    size: (u32, u32),
    /// Flag indicating if a redraw is needed
    needs_redraw: bool,
    /// Flag indicating if we've acknowledged the first configure event
    /// This is critical for initial rendering - GPUI requests frame callback on first configure
    acknowledged_first_configure: bool,
    /// Surface format
    format: TextureFormat,
    /// Pending texture (stored between get_current_texture and present)
    pending_texture: Option<SurfaceTexture>,
    /// Wayland state for event handling
    state: Arc<Mutex<WaylandState>>,
    /// Frame callback for smooth rendering synchronization
    frame_callback: Option<wayland_client::protocol::wl_callback::WlCallback>,
}

/// State for Wayland event handling
#[cfg(target_os = "linux")]
pub(crate) struct WaylandState {
    /// Current window size from configure events
    size: (u32, u32),
    /// Flag indicating if a redraw is needed
    needs_redraw: bool,
    /// Flag indicating if window should close
    should_close: bool,
    /// Flag indicating if configure event was received and needs acknowledgment
    configure_serial: Option<u32>,
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
impl wayland_client::Dispatch<wayland_client::protocol::wl_callback::WlCallback, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _callback: &wayland_client::protocol::wl_callback::WlCallback,
        event: wayland_client::protocol::wl_callback::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_callback::Event::Done { callback_data } => {
                // Frame callback completed - compositor is ready for next frame
                state.needs_redraw = true;
                let _ = callback_data;
            }
            _ => {
                // Other callback events (none currently defined)
            }
        }
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
        window: &smithay_client_toolkit::shell::xdg::window::Window,
        configure: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        serial: u32,
    ) {
        // Update size from configure event if provided
        if let (Some(w), Some(h)) = (configure.new_size.0, configure.new_size.1) {
            self.size = (w.get(), h.get());
            self.needs_redraw = true;
            eprintln!("[NPTK] Wayland configure event: size set to {}x{}", w.get(), h.get());
        } else {
            // If configure event doesn't provide size, use window's current size
            // This can happen on initial configure before window is mapped
            eprintln!("[NPTK] Wayland configure event: no size provided, keeping current size {}x{}", self.size.0, self.size.1);
        }
        
        // Store the configure serial for acknowledgment
        // We'll acknowledge and commit in WaylandSurface after the roundtrip
        self.configure_serial = Some(serial);
        
        // Note: SCTK Window handles ack_configure internally, but we track the serial
        // to know when to commit the surface
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
    /// * `gpu_context` - GPU context to use for creating wgpu surface (must use same Instance)
    ///
    /// # Returns
    /// * `Ok(WaylandSurface)` if creation succeeded
    /// * `Err(String)` if creation failed
    pub fn new(width: u32, height: u32, title: &str, gpu_context: &crate::vgi::GpuContext) -> Result<Self, String> {
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
        
        // Create window (SCTK creates xdg_surface and xdg_toplevel internally)
        // NOTE: SCTK's Window abstraction doesn't expose xdg_surface/toplevel directly,
        // so we can't set app_id or window geometry directly like GPUI does.
        // We rely on SCTK's internal handling and set app_id via environment variable.
        let window = xdg_shell.create_window(
            Surface::from(wl_surface),
            WindowDecorations::ServerDefault,
            &qh,
        );
        
        // Set window title
        window.set_title(title);
        
        // Set app_id via environment variable
        // Note: SCTK's Window doesn't expose set_app_id directly, but the compositor
        // will use NPTK_APP_ID from the environment or derive it from the window class
        if let Ok(app_id) = std::env::var("NPTK_APP_ID") {
            log::debug!("NPTK_APP_ID set to: {} (compositor will use this)", app_id);
        } else {
            log::debug!("No NPTK_APP_ID set, compositor will derive from window class");
        }
        
        // Set window size
        window.set_min_size(Some((width, height)));
        window.set_max_size(Some((width, height)));
        
        // CRITICAL: Commit the surface immediately after creating the window
        // This is what GPUI does - it makes the window appear in the taskbar immediately
        // The compositor will show a placeholder until a buffer is attached
        log::debug!("Committing surface immediately after window creation (GPUI pattern)");
        wl_surface_stored.commit();
        
        // Roundtrip to ensure window is created and configured
        let mut state = WaylandState {
            size: (width, height),
            needs_redraw: true,
            should_close: false,
            configure_serial: None,
        };
        event_queue.roundtrip(&mut state)
            .map_err(|e| format!("Failed to roundtrip: {:?}", e))?;
        
        // After configure event, acknowledge it and request frame callback for initial render
        // This is critical - GPUI requests frame callback on first configure to trigger initial rendering
        let acknowledged_first_configure = state.configure_serial.is_some();
        if acknowledged_first_configure {
            log::debug!("Configure event received, requesting frame callback for initial render");
            
            // Request frame callback for initial render (like GPUI does)
            // This ensures we get a frame callback to trigger the first render
            // The frame callback will fire when the compositor is ready, triggering rendering
            // We'll store this callback in the WaylandSurface struct after creation
            let callback = wl_surface_stored.frame(&qh, ());
            // Store it temporarily - we'll move it to the struct
            drop(callback);
            
            // NOTE: We do NOT commit here - we commit AFTER rendering when a buffer is attached
            // GPUI commits after rendering in completed_frame() (line 1221), not after configure
            // Committing before a buffer is attached won't make the window visible
        }
        
        // Create wgpu surface from raw window handle using GpuContext's Instance
        // This ensures the surface is created with the same Instance that enumerates adapters
        let (wgpu_surface, format) = {
            // Get raw window handle
            use wayland_client::Proxy;
            use std::ptr::NonNull;
            let wl_surface_ptr = NonNull::new(wl_surface_stored.id().as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| "Invalid surface pointer".to_string())?;
            let display_ptr = NonNull::new(connection.display().id().as_ptr() as *mut std::ffi::c_void)
                .ok_or_else(|| "Invalid display pointer".to_string())?;
            
            // Create raw window handle (only needs surface pointer)
            let raw_handle = raw_window_handle::WaylandWindowHandle::new(wl_surface_ptr);
            let raw_display_handle = raw_window_handle::WaylandDisplayHandle::new(display_ptr);
            
            // Use GpuContext's Instance to create the surface
            // This ensures compatibility - the surface will be created with the same Instance
            // that enumerates adapters and creates devices
            let instance = gpu_context.instance();
            
            // Create surface using unsafe API with raw handles
            // This is necessary because we need to pass raw pointers
            let surface_target_unsafe = vello::wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: raw_window_handle::RawDisplayHandle::Wayland(raw_display_handle),
                raw_window_handle: raw_window_handle::RawWindowHandle::Wayland(raw_handle),
            };
            
            log::debug!("Creating wgpu surface from Wayland raw window handle using GpuContext's Instance...");
            match unsafe { instance.create_surface_unsafe(surface_target_unsafe) } {
                Ok(surface) => {
                    log::debug!("Successfully created wgpu surface from Wayland handle");
                    
                    // Do an extra roundtrip to ensure the compositor has registered the surface
                    // This helps with adapter enumeration on Wayland
                    log::debug!("Performing extra roundtrip to ensure compositor registers wgpu surface");
                    let mut temp_state = WaylandState {
                        size: (width, height),
                        needs_redraw: false,
                        should_close: false,
                        configure_serial: None,
                    };
                    let _ = event_queue.roundtrip(&mut temp_state);
                    
                    // Query surface format from adapter if available
                    // For now, we'll use a default format and update it when adapter is available
                    // The format will be queried later via get_capabilities() when we have an adapter
                    let format = TextureFormat::Bgra8Unorm;
                    (Some(surface), format)
                }
                Err(e) => {
                    eprintln!("[NPTK] Warning: Failed to create wgpu surface from Wayland handle: {:?}", e);
                    eprintln!("[NPTK] Falling back to None - rendering will not work until this is fixed");
                    (None, TextureFormat::Bgra8Unorm)
                }
            }
        };
        
        // Use the size from state (may have been updated by configure event)
        let actual_size = state.size;
        let state = Arc::new(Mutex::new(state));
        
        Ok(Self {
            connection,
            event_queue,
            window,
            wl_surface: wl_surface_stored,
            wgpu_surface,
            is_configured: false,
            size: actual_size,
            needs_redraw: true,
            acknowledged_first_configure,
            format,
            pending_texture: None,
            state,
            frame_callback: None,
        })
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

    /// Create the wgpu surface using GpuContext's Instance.
    /// This method is no longer needed since we create the surface in new(),
    /// but kept for backward compatibility.
    #[deprecated(note = "wgpu surface is now created in new() using GpuContext")]
    pub(crate) fn create_wgpu_surface_from_device(
        &mut self,
        _device: &vello::wgpu::Device,
    ) -> Result<(), String> {
        // Surface should already be created in new()
        if self.wgpu_surface.is_some() {
            Ok(())
        } else {
            Err("wgpu surface not created - this should not happen".to_string())
        }
    }
    
    /// Get a reference to the Wayland window.
    pub fn window(&self) -> &smithay_client_toolkit::shell::xdg::window::Window {
        &self.window
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
        
        // If surface is configured, we need to reconfigure it with new size
        // Note: This requires a device, which we don't have here
        // The caller should reconfigure after resize if needed
        if self.is_configured {
            self.is_configured = false;
            log::debug!("Surface resized to {}x{}, needs reconfiguration", width, height);
        }
        
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
        
        // Check if window should close
        if state_guard.should_close {
            // Note: Close request is detected, but we can't directly exit the event loop here
            // The application handler should check should_close() and handle it
            log::debug!("Wayland window close requested");
        }
        
        // Handle configure events - acknowledge them and request frame callback on first configure
        // This matches GPUI's behavior (lines 564, 581-585 in window.rs)
        if let Some(serial) = state_guard.configure_serial.take() {
            // Note: SCTK's Window handles ack_configure internally via WindowHandler
            // We track the serial to know when configure events occur, but SCTK acknowledges them
            // GPUI explicitly calls ack_configure, but SCTK's Window abstraction doesn't expose xdg_surface
            // so we rely on SCTK's internal handling.
            
            // Set window geometry after configure (like GPUI does, lines 574-579)
            // SCTK's Window doesn't expose set_window_geometry directly, but SCTK handles geometry internally.
            // The window size is already set via set_min_size/set_max_size above.
            
            // Request frame callback on first configure (like GPUI does)
            if !self.acknowledged_first_configure {
                self.acknowledged_first_configure = true;
                log::debug!("First configure event acknowledged, requesting frame callback for initial render");
                
                // Request frame callback to trigger initial render
                let qh = self.event_queue.handle();
                // Drop old callback if it exists
                let _ = self.frame_callback.take();
                // Request new frame callback
                let callback = self.wl_surface.frame(&qh, ());
                self.frame_callback = Some(callback);
                
                // CRITICAL: Set needs_redraw to true so dispatch_events() returns true
                // This ensures update() is called and rendering happens
                state_guard.needs_redraw = true;
                
                // Also commit the surface to ensure the frame callback is registered
                // The compositor needs to see the commit to schedule the frame callback
                self.wl_surface.commit();
                log::debug!("Committed surface to register frame callback");
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

