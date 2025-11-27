#![cfg(target_os = "linux")]
#![allow(missing_docs)]

//! Native Wayland surface implementation backed by a shared event loop.

use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{self, SurfaceTexture, TextureView};

use wayland_client::protocol::{
    wl_compositor, wl_keyboard, wl_pointer, wl_region, wl_shm, wl_surface,
};
use wayland_client::{Connection, Proxy};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};
use wayland_protocols_plasma::server_decoration::client::org_kde_kwin_server_decoration;

use super::surface::{OffscreenSurface, SurfaceTrait};
use crate::vgi::wayland::{WaylandClient, WaylandQueueHandle};
// Re-export event types for backward compatibility with existing code
pub(crate) use crate::vgi::wayland::events::{InputEvent, KeyboardEvent, PointerEvent};

// Use WaylandSurfaceInner from the new modular wayland module
pub(crate) use crate::vgi::wayland::surface::WaylandSurfaceInner;


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
        log::debug!("WaylandSurface::new queue_handle_ptr={:p}", &queue_handle);

        let wl_surface: wl_surface::WlSurface =
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

        // Appmenu setup is handled by the menubar module via VGI's public appmenu API.
        // When menu info becomes available, the menubar module will call
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
        })
    }

    fn create_wgpu_surface(
        connection: &Connection,
        wl_surface: &wl_surface::WlSurface,
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
            log::debug!("WaylandSurface::dispatch_events: got {} input events", new_events.len());
            for event in &new_events {
                if let InputEvent::Keyboard(_) = event {
                    log::debug!("WaylandSurface::dispatch_events: keyboard event: {:?}", event);
                }
            }
            self.pending_input_events.append(&mut new_events);
        }

        Ok(self.needs_redraw)
    }
}

impl WaylandSurface {
    /// Get the Wayland surface protocol ID (surface key).
    pub fn surface_key(&self) -> u32 {
        self.inner.surface_key()
    }

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

    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    pub fn has_received_configure(&self) -> bool {
        self.first_configure_seen
    }

    pub fn first_frame_seen(&self) -> bool {
        self.inner.get_first_frame_seen()
    }

    pub fn should_close(&self) -> bool {
        self.inner.get_should_close()
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

    pub fn prepare_frame(&self) {
        if !self.is_configured {
            return;
        }
        self.inner.prepare_frame_callback();
    }
}
