#![cfg(target_os = "linux")]

//! Wayland surface lifecycle and buffer management.

use std::sync::{Arc, Mutex};

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

    pub fn push_input_event(&self, event: InputEvent) {
        let mut state = self.state.lock().unwrap();
        state.input_events.push(event);
    }

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

    pub fn surface_key(&self) -> u32 {
        self.surface_key
    }
    
    #[cfg(feature = "global-menu")]
    pub fn wl_surface(&self) -> &wl_surface::WlSurface {
        &self.wl_surface
    }

    pub fn handle_toplevel_configure(&self, width: i32, height: i32) {
        log::debug!("Wayland toplevel configure: {}x{}", width, height);
        if width <= 0 || height <= 0 {
            return;
        }
        let mut state = self.state.lock().unwrap();
        state.pending_size = Some((width as u32, height as u32));
    }

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

    pub fn handle_frame_done(&self) {
        log::trace!("Wayland frame callback done");
        let mut state = self.state.lock().unwrap();
        state.frame_callback = None;
        state.first_frame_seen = true;
    }

    pub fn prepare_frame_callback(&self) {
        let mut state = self.state.lock().unwrap();
        self.ensure_frame_callback_locked(&mut state);
    }

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
        surface_key: u32,
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
        let mut file = tempfile::tempfile().map_err(|e| format!("tempfile failed: {:?}", e))?;
        file.set_len(size_bytes as u64)
            .map_err(|e| format!("ftruncate failed: {:?}", e))?;

        // Fill with opaque gray
        let mut pixels = vec![0u8; size_bytes as usize];
        for px in pixels.chunks_exact_mut(4) {
            px[0] = 0x80; // B
            px[1] = 0x80; // G
            px[2] = 0x80; // R
            px[3] = 0xFF; // A
        }
        file.write_all(&pixels)
            .map_err(|e| format!("write failed: {:?}", e))?;

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

    pub fn mark_closed(&self) {
        let mut state = self.state.lock().unwrap();
        state.should_close = true;
    }

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
            log::trace!("Registered wl_surface.frame callback {} for surface {}", callback_id, self.surface_key);
        }
    }

    pub fn request_redraw(&self) {
        let mut state = self.state.lock().unwrap();
        state.needs_redraw = true;
        self.ensure_frame_callback_locked(&mut state);
    }

    pub fn take_status(&self) -> SurfaceStatus {
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

    pub fn has_acknowledged_initial_configure(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.first_configure_acked
    }

    pub(crate) fn get_size(&self) -> (u32, u32) {
        let state = self.state.lock().unwrap();
        state.size
    }

    pub(crate) fn get_first_frame_seen(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.first_frame_seen
    }

    pub(crate) fn get_should_close(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.should_close
    }
}

pub(crate) struct SurfaceStatus {
    pub size: (u32, u32),
    pub needs_redraw: bool,
    pub configured: bool,
    pub should_close: bool,
}

