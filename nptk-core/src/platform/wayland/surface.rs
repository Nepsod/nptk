#![cfg(target_os = "linux")]

//! Wayland surface lifecycle and buffer management.

use std::sync::Arc;
use std::sync::Mutex;

use wayland_client::protocol::{wl_compositor, wl_region, wl_shm, wl_surface};
use wayland_client::Proxy;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};

use super::client::{WaylandClient, WaylandQueueHandle};
use super::events::InputEvent;

#[derive(Default)]
struct SurfaceState {
    size: (u32, u32),
    pending_size: Option<(u32, u32)>,
    needs_redraw: bool,
    configured: bool,
    should_close: bool,
    frame_callback: Option<wayland_client::protocol::wl_callback::WlCallback>,
    first_frame_seen: bool,
    fallback_committed: bool,
    input_events: Vec<InputEvent>,
    first_configure_acked: bool,
    frame_ready: bool,
}

/// Internal Wayland surface state.
pub struct WaylandSurfaceInner {
    surface_key: u32,
    pub(crate) wl_surface: wl_surface::WlSurface,
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    #[allow(dead_code)]
    pub(crate) _decoration: Option<wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    #[allow(dead_code)]
    pub(crate) _kde_decoration: Option<wayland_protocols_plasma::server_decoration::client::org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration>,
    queue_handle: WaylandQueueHandle,
    state: Mutex<SurfaceState>,
    compositor: wl_compositor::WlCompositor,
}

impl WaylandSurfaceInner {
    /// Create a new Wayland surface inner state.
    ///
    /// Initializes the surface with the given Wayland objects and initial size.
    pub fn new(
        wl_surface: wl_surface::WlSurface,
        xdg_surface: xdg_surface::XdgSurface,
        xdg_toplevel: xdg_toplevel::XdgToplevel,
        queue_handle: WaylandQueueHandle,
        initial_size: (u32, u32),
        compositor: wl_compositor::WlCompositor,
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
            compositor,
        })
    }

    /// Push an input event to the surface's event queue.
    pub fn push_input_event(&self, event: InputEvent) {
        let mut state = self.state.lock().unwrap();
        state.input_events.push(event);
    }

    /// Take all pending input events from the surface.
    ///
    /// Returns a vector of all accumulated input events and clears the internal queue.
    pub fn take_input_events(&self) -> Vec<InputEvent> {
        let mut state = self.state.lock().unwrap();
        if state.input_events.is_empty() {
            Vec::new()
        } else {
            let mut events = Vec::with_capacity(state.input_events.len());
            events.append(&mut state.input_events);
            events
        }
    }

    /// Get the surface's protocol ID.
    pub fn surface_key(&self) -> u32 {
        self.surface_key
    }

    /// Get the underlying Wayland surface object.
    #[cfg(feature = "global-menu")]
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.wl_surface
    }

    /// Handle a toplevel configure event.
    ///
    /// Stores the pending size to be applied after acknowledging the configure.
    pub fn handle_toplevel_configure(&self, width: i32, height: i32) {
        log::debug!("Wayland toplevel configure: {}x{}", width, height);
        if width <= 0 || height <= 0 {
            return;
        }
        let mut state = self.state.lock().unwrap();
        state.pending_size = Some((width as u32, height as u32));
    }

    /// Handle configure acknowledgment.
    ///
    /// Applies the pending size after acknowledging the configure event.
    pub fn handle_configure_after_ack(&self, serial: u32) {
        let xdg_id = self.xdg_surface.id().protocol_id();
        let wl_id = self.wl_surface.id().protocol_id();
        log::debug!(
            "Wayland xdg_surface post-ack: serial={} on xdg_surface#{}",
            serial,
            xdg_id
        );
        log::trace!(
            "Wayland configure post-ack serial={} xdg_surface#{} wl_surface#{}",
            serial,
            xdg_id,
            wl_id
        );
        let mut state = self.state.lock().unwrap();
        let mut size = state.pending_size.take().unwrap_or_else(|| state.size);

        // Fallback if compositor reports 0x0 - choose a default to ensure mapping
        if size.0 == 0 || size.1 == 0 {
            size = (800, 600);
            log::debug!(
                "Wayland configure reported 0x0; using fallback size {}x{}",
                size.0,
                size.1
            );
        }
        let width = size.0.max(1);
        let height = size.1.max(1);

        log::trace!("Wayland geometry set to {}x{}", width, height);
        self.xdg_surface
            .set_window_geometry(0, 0, width as i32, height as i32);

        // Update opaque region to match the new buffer size so compositors can treat it as opaque.
        let region: wl_region::WlRegion = self.compositor.create_region(&self.queue_handle, ());
        region.add(0, 0, width as i32, height as i32);
        self.wl_surface.set_opaque_region(Some(&region));
        region.destroy();

        // Only use the fallback SHM buffer until the GPU has submitted a frame.
        let should_attach_fallback = !state.first_frame_seen && !state.fallback_committed;
        if should_attach_fallback {
            state.fallback_committed = true;
        }
        drop(state);
        if should_attach_fallback {
            if let Some(ref shm) = WaylandClient::instance().globals().shm {
                if let Err(err) = Self::attach_first_shm_buffer(
                    &self.wl_surface,
                    shm,
                    &self.queue_handle,
                    self.surface_key,
                    width,
                    height,
                ) {
                    log::warn!("Failed to attach first SHM buffer on configure: {}", err);
                    log::warn!("Wayland first-present fallback failed: {}", err);
                }
            }
        }

        let mut state = self.state.lock().unwrap();
        state.size = (width, height);
        state.configured = true;
        state.needs_redraw = true;
        state.first_configure_acked = true;
        log::debug!(
            "Wayland configure applied: size={}x{}, set configured=true, needs_redraw=true",
            width,
            height
        );

        self.ensure_frame_callback_locked(&mut state);
    }

    /// Handle frame callback completion.
    ///
    /// Called when the compositor signals that a frame has been presented.
    pub fn handle_frame_done(&self) {
        log::trace!("Wayland frame callback done");
        let mut state = self.state.lock().unwrap();
        state.frame_callback = None;
        state.first_frame_seen = true;
        state.frame_ready = true;
    }

    /// Prepare a frame callback for the next frame.
    ///
    /// Requests a callback from the compositor to pace frame rendering.
    pub fn prepare_frame_callback(&self) {
        let mut state = self.state.lock().unwrap();
        self.ensure_frame_callback_locked(&mut state);
    }

    /// Set a pending size change for the surface.
    ///
    /// The size will be applied on the next configure acknowledgment.
    pub fn set_pending_size(&self, width: u32, height: u32) {
        let mut state = self.state.lock().unwrap();
        state.pending_size = Some((width, height));
        state.needs_redraw = true;
        self.ensure_frame_callback_locked(&mut state);
    }

    fn attach_first_shm_buffer(
        wl_surface: &wl_surface::WlSurface,
        shm: &wl_shm::WlShm,
        queue_handle: &WaylandQueueHandle,
        _surface_key: u32,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        use std::io::Write;
        use std::os::fd::AsFd;
        let stride = (width * 4) as i32;
        let size_bytes = (stride as u32) * height;
        log::trace!(
            "Wayland fallback SHM buffer {}x{} stride {}",
            width,
            height,
            stride
        );
        // Use smol::unblock for tempfile operations to avoid blocking the runtime
        let file = smol::block_on(async {
            let file = smol::unblock(|| tempfile::tempfile())
                .await
                .map_err(|e| format!("tempfile failed: {:?}", e))?;
            
            let file = smol::unblock({
                let size = size_bytes as u64;
                move || {
                    let mut f = file;
                    f.set_len(size).map_err(|e| format!("ftruncate failed: {:?}", e))?;
                    Ok::<std::fs::File, String>(f)
                }
            })
            .await?;

            // Fill with opaque gray
            let mut pixels = vec![0u8; size_bytes as usize];
            for px in pixels.chunks_exact_mut(4) {
                px[0] = 0x80; // B
                px[1] = 0x80; // G
                px[2] = 0x80; // R
                px[3] = 0xFF; // A
            }
            
            let file = smol::unblock({
                let pixels = pixels.clone();
                move || {
                    use std::io::Write;
                    let mut f = file;
                    f.write_all(&pixels).map_err(|e| format!("write failed: {:?}", e))?;
                    Ok::<std::fs::File, String>(f)
                }
            })
            .await?;
            
            Ok::<std::fs::File, String>(file)
        })?;

        let pool = shm.create_pool(file.as_fd(), size_bytes as i32, queue_handle, ());
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride,
            wl_shm::Format::Argb8888,
            queue_handle,
            (),
        );

        log::trace!("Wayland fallback attach buffer");
        wl_surface.attach(Some(&buffer), 0, 0);
        log::trace!("Wayland fallback damage {}x{}", width, height);
        wl_surface.damage_buffer(0, 0, width as i32, height as i32);
        // Frame BEFORE commit so we get paced correctly
        log::trace!("Wayland fallback frame request");
        let _ = wl_surface.frame(queue_handle, ());
        log::trace!("Wayland fallback commit");
        wl_surface.commit();
        log::trace!("Wayland fallback flush after commit");
        let _ = WaylandClient::instance().connection().flush();
        Ok(())
    }

    /// Mark the surface as closed.
    ///
    /// Signals that the surface should be closed.
    pub fn mark_closed(&self) {
        let mut state = self.state.lock().unwrap();
        state.should_close = true;
    }

    /// Called after presenting a frame.
    ///
    /// Marks that a redraw is no longer needed.
    pub fn after_present(&self) {
        let mut state = self.state.lock().unwrap();
        state.needs_redraw = false;
    }

    fn ensure_frame_callback_locked(&self, state: &mut SurfaceState) {
        if state.frame_callback.is_none() {
            let callback = self.wl_surface.frame(&self.queue_handle, ());
            let callback_id = callback.id().protocol_id();
            // Register callback -> surface mapping
            use super::client::WaylandClient;
            WaylandClient::instance().register_callback(callback_id, self.surface_key);
            state.frame_callback = Some(callback);
            if let Err(err) = WaylandClient::instance().connection().flush() {
                log::warn!(
                    "Failed to flush Wayland connection after frame request: {:?}",
                    err
                );
            }
            log::trace!(
                "Registered wl_surface.frame callback {} for surface {}",
                callback_id,
                self.surface_key
            );
        }
    }

    /// Request a redraw of the surface.
    ///
    /// Marks the surface as needing a redraw and ensures a frame callback is set up.
    pub fn request_redraw(&self) {
        let mut state = self.state.lock().unwrap();
        state.needs_redraw = true;
        self.ensure_frame_callback_locked(&mut state);
    }

    /// Take the current surface status.
    ///
    /// Returns the current status and resets the needs_redraw and configured flags.
    pub fn take_status(&self) -> SurfaceStatus {
        let mut state = self.state.lock().unwrap();
        let status = SurfaceStatus {
            size: state.size,
            needs_redraw: state.needs_redraw,
            configured: state.configured,
            should_close: state.should_close,
            frame_ready: state.frame_ready,
        };
        state.needs_redraw = false;
        state.configured = false;
        state.frame_ready = false;
        status
    }

    /// Check if the initial configure has been acknowledged.
    pub fn has_acknowledged_initial_configure(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.first_configure_acked
    }

    /// Get the current surface size.
    pub(crate) fn get_size(&self) -> (u32, u32) {
        let state = self.state.lock().unwrap();
        state.size
    }

    /// Check if the first frame has been seen.
    pub(crate) fn get_first_frame_seen(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.first_frame_seen
    }

    /// Check if the surface should be closed.
    pub(crate) fn get_should_close(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.should_close
    }
}

/// Surface status snapshot.
///
/// Contains the current state of a Wayland surface.
pub(crate) struct SurfaceStatus {
    /// Current surface size.
    pub size: (u32, u32),
    /// Whether the surface needs a redraw.
    pub needs_redraw: bool,
    /// Whether the surface has been configured.
    pub configured: bool,
    /// Whether the surface should be closed.
    pub should_close: bool,
    /// Whether a frame has been presented since the last check.
    pub frame_ready: bool,
}

// WaylandSurface implementation (merged from vgi/wayland_surface.rs)
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use std::ptr::NonNull;
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{self, SurfaceTexture, TextureView};
use wayland_client::Connection;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;

// Import SurfaceTrait and OffscreenSurface from VGI
use crate::vgi::surface::{OffscreenSurface, SurfaceTrait};

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
    first_configure_seen: bool,
    pending_input_events: Vec<InputEvent>,
    offscreen: Option<OffscreenSurface>,
    blitter: Option<TextureBlitter>,
    frame_ready: bool,
}

impl WaylandSurface {
    /// Create a new Wayland surface.
    ///
    /// Initializes a new Wayland surface with the given dimensions and title.
    /// If appmenu manager is available and menu info is already set, appmenu will be
    /// configured immediately, before window focus.
    pub fn new(
        width: u32,
        height: u32,
        title: &str,
        gpu_context: &crate::vgi::GpuContext,
    ) -> Result<Self, String> {
        let client = WaylandClient::instance();
        let globals = client.globals();
        let queue_handle = client.queue_handle();
        log::debug!("WaylandSurface::new queue_handle_ptr={:p}", &queue_handle);

        let wl_surface: wayland_client::protocol::wl_surface::WlSurface =
            globals.compositor.create_surface(&queue_handle, ());
        let surface_key = wl_surface.id().protocol_id();
        log::debug!(
            "Wayland create wl_surface#{} (surface_key={})",
            wl_surface.id().protocol_id(),
            surface_key
        );
        let xdg_surface = globals
            .wm_base
            .get_xdg_surface(&wl_surface, &queue_handle, ());
        log::debug!(
            "Wayland create xdg_surface#{} (for wl_surface#{})",
            xdg_surface.id().protocol_id(),
            wl_surface.id().protocol_id()
        );
        let xdg_toplevel = xdg_surface.get_toplevel(&queue_handle, ());
        log::debug!(
            "Wayland create xdg_toplevel#{} (for xdg_surface#{})",
            xdg_toplevel.id().protocol_id(),
            xdg_surface.id().protocol_id()
        );

        xdg_toplevel.set_title(title.to_owned());
        xdg_toplevel.set_app_id("com.nptk.app".to_owned());

        let mut inner = WaylandSurfaceInner::new(
            wl_surface.clone(),
            xdg_surface.clone(),
            xdg_toplevel.clone(),
            queue_handle.clone(),
            (width.max(1), height.max(1)),
            globals.compositor.clone(),
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

        // Try to set appmenu immediately if both appmenu_manager and menu info are available
        #[cfg(feature = "global-menu")]
        {
            if client.globals().appmenu_manager.is_some() {
                if let Some((service, path)) = crate::platform::MenuInfoStorage::get() {
                    if let Err(e) =
                        client.set_appmenu_for_surface_with_info(&wl_surface, service, path)
                    {
                        log::debug!(
                            "Failed to set appmenu immediately on surface creation: {}",
                            e
                        );
                    } else {
                        log::info!(
                            "Appmenu set immediately on surface creation (before window focus)"
                        );
                    }
                } else {
                    log::debug!("Appmenu manager available but menu info not yet set - will be set when menu info becomes available");
                }
            }
        }

        // Commit the surface after registering so we can handle configure events
        wl_surface.commit();
        // Flush the Connection (not the event queue) so compositor sees the commit
        let _ = client.connection().flush();
        log::trace!(
            "Wayland initial commit (no buffer) on wl_surface#{}",
            wl_surface.id().protocol_id()
        );

        client.wait_for_initial_configure(surface_key)?;

        let connection = client.connection();
        let (wgpu_surface, format) =
            Self::create_wgpu_surface(&connection, &wl_surface, gpu_context)?;

        // Appmenu setup: If menu info is already available, appmenu is set immediately above.
        // If menu info becomes available later, the menubar module will call
        // appmenu::update_appmenu_for_all_surfaces() which will set appmenu for this surface.

        Ok(Self {
            client,
            inner,
            wgpu_surface,
            format,
            size: (width.max(1), height.max(1)),
            is_configured: false,
            needs_redraw: false,
            pending_reconfigure: false,
            first_configure_seen: false,
            pending_input_events: Vec::new(),
            offscreen: None,
            blitter: None,
            frame_ready: false,
        })
    }

    fn create_wgpu_surface(
        connection: &Connection,
        wl_surface: &wayland_client::protocol::wl_surface::WlSurface,
        gpu_context: &crate::vgi::GpuContext,
    ) -> Result<(Option<wgpu::Surface<'static>>, wgpu::TextureFormat), String> {
        let surface_ptr = NonNull::new(wl_surface.id().as_ptr() as *mut std::ffi::c_void)
            .ok_or_else(|| "Invalid Wayland surface pointer".to_string())?;
        let display_ptr = NonNull::new(connection.display().id().as_ptr() as *mut std::ffi::c_void)
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
        self.client.unregister_surface(self.inner.surface_key());

        // Proactively destroy Wayland objects so the compositor closes the popup.
        // Destroy toplevel first, then xdg_surface, then wl_surface.
        // Ignore errors; the protocol objects may already be gone.
        #[allow(unused_must_use)]
        {
            self.inner.xdg_toplevel.destroy();
            self.inner.xdg_surface.destroy();
            self.inner.wl_surface.destroy();
        }
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
        log::debug!("Wayland present(): post-present maintenance");
        self.inner.after_present();
        self.needs_redraw = false;
        if let Err(err) = self.client.connection().flush() {
            log::warn!("Wayland flush error after present: {:?}", err);
        }
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.inner.set_pending_size(width.max(1), height.max(1));
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
        log::trace!(
            "WaylandSurface::dispatch_events called (surface_key={})",
            self.inner.surface_key()
        );
        // Drive Wayland event processing on the owning thread.
        self.client.dispatch_pending()?;

        let status = self.inner.take_status();
        if self.size != status.size {
            log::debug!(
                "Wayland dispatch: size changed to {}x{}",
                status.size.0,
                status.size.1
            );
        }
        self.size = status.size;
        if status.configured {
            self.pending_reconfigure = true;
            self.first_configure_seen = true;
            self.is_configured = false;
            log::debug!("Wayland dispatch: configured event received; pending_reconfigure=true");
        }
        if status.needs_redraw {
            self.needs_redraw = true;
            log::trace!("Wayland dispatch: needs_redraw=true");
        }
        if status.frame_ready {
            self.frame_ready = true;
        }

        // If we just got configured and require reconfiguration, request a redraw immediately
        // so the higher layers render once and present a buffer to get mapped.
        if self.is_configured && self.pending_reconfigure {
            self.needs_redraw = true;
            log::debug!("Wayland dispatch: forcing redraw after configure");
        }

        if status.should_close {
            self.needs_redraw = false;
            return Err("Wayland surface requested close".to_string());
        }

        let mut new_events = self.inner.take_input_events();
        if !new_events.is_empty() {
            log::debug!(
                "WaylandSurface::dispatch_events: got {} input events",
                new_events.len()
            );
            for event in &new_events {
                if let InputEvent::Keyboard(_) = event {
                    log::debug!(
                        "WaylandSurface::dispatch_events: keyboard event: {:?}",
                        event
                    );
                }
            }
            self.pending_input_events.append(&mut new_events);
        }
 
        Ok(self.needs_redraw)
    }

    fn take_frame_ready(&mut self) -> bool {
        let ready = self.frame_ready;
        self.frame_ready = false;
        ready
    }
}

impl WaylandSurface {
    /// Get the Wayland surface protocol ID (surface key).
    pub fn surface_key(&self) -> u32 {
        self.inner.surface_key()
    }

    /// Configure the wgpu surface.
    ///
    /// Sets up the surface configuration with the given format and present mode.
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
        let size = self.inner.get_size();

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
        self.offscreen = Some(OffscreenSurface::new(device, size.0.max(1), size.1.max(1)));
        self.blitter = Some(TextureBlitter::new(device, format));
        Ok(())
    }

    /// Create a render view for offscreen rendering.
    ///
    /// Creates or updates the offscreen render target for the given dimensions.
    pub fn create_render_view(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<TextureView, String> {
        let target_width = width.max(1);
        let target_height = height.max(1);
        if self
            .offscreen
            .as_ref()
            .map(|rt| rt.size() != (target_width, target_height))
            .unwrap_or(true)
        {
            self.offscreen = Some(OffscreenSurface::new(device, target_width, target_height));
        }

        Ok(self
            .offscreen
            .as_ref()
            .expect("offscreen render target should exist")
            .create_view())
    }

    /// Blit the rendered content to the surface.
    ///
    /// Copies the offscreen render target to the surface texture.
    pub fn blit_to_surface(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &TextureView,
        target: &TextureView,
    ) -> Result<(), String> {
        if let Some(blitter) = &self.blitter {
            blitter.copy(device, encoder, source, target);
            Ok(())
        } else {
            Err("Wayland surface is not configured".to_string())
        }
    }

    /// Check if the surface is configured.
    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    /// Check if the surface has received at least one configure event.
    pub fn has_received_configure(&self) -> bool {
        self.first_configure_seen
    }

    /// Check if the first frame has been presented.
    pub fn first_frame_seen(&self) -> bool {
        self.inner.get_first_frame_seen()
    }

    /// Check if the surface should be closed.
    pub fn should_close(&self) -> bool {
        self.inner.get_should_close()
    }

    /// Check if the surface requires reconfiguration.
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

    /// Prepare for the next frame.
    ///
    /// Requests a frame callback from the compositor to pace rendering.
    pub fn prepare_frame(&self) {
        if !self.is_configured {
            return;
        }
        self.inner.prepare_frame_callback();
    }
}
