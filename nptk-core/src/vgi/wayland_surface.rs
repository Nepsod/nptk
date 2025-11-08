#![cfg(target_os = "linux")]

//! Native Wayland surface implementation backed by a shared event loop.

use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use vello::wgpu::{self, SurfaceTexture};

use wayland_client::protocol::wl_surface;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};
use wayland_client::{Connection, Proxy};

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
}

pub(crate) struct WaylandSurfaceInner {
    surface_key: u32,
    wl_surface: wl_surface::WlSurface,
    xdg_surface: xdg_surface::XdgSurface,
    xdg_toplevel: xdg_toplevel::XdgToplevel,
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
            queue_handle,
            state: Mutex::new(state),
        })
    }

    pub(crate) fn surface_key(&self) -> u32 {
        self.surface_key
    }

    pub(crate) fn handle_toplevel_configure(&self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let mut state = self.state.lock().unwrap();
        state.pending_size = Some((width as u32, height as u32));
    }

    pub(crate) fn handle_configure(&self, serial: u32) {
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

        state.size = (width, height);
        state.configured = true;
        state.needs_redraw = true;

        self.ensure_frame_callback_locked(&mut state);
    }

    pub(crate) fn handle_frame_done(&self) {
        let mut state = self.state.lock().unwrap();
        state.frame_callback = None;
        state.needs_redraw = true;
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
        }
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

        wl_surface.commit();

        let inner = WaylandSurfaceInner::new(
            wl_surface.clone(),
            xdg_surface.clone(),
            xdg_toplevel.clone(),
            queue_handle.clone(),
            (width.max(1), height.max(1)),
        );
        client.register_surface(&inner);

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
        self.inner.after_present();
        self.inner.wl_surface().commit();
        self.needs_redraw = false;
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
        let status = self.inner.take_status();
        self.size = status.size;
        self.is_configured = status.configured;
        self.needs_redraw = status.needs_redraw;

        if status.should_close {
            self.needs_redraw = false;
            return Err("Wayland surface requested close".to_string());
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: self.size.0,
            height: self.size.1,
            present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        wgpu_surface.configure(device, &config);
        self.format = format;
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        self.is_configured
    }

    pub fn should_close(&self) -> bool {
        let state = self.inner.state.lock().unwrap();
        state.should_close
    }
}

