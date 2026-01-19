#![allow(unused_variables)]
//! Platform-agnostic surface abstraction for rendering.
//!
//! This module provides a unified surface interface that can work with different
//! platform backends (Winit, native Wayland, and potentially others).

#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::platform::wayland::events::InputEvent;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::platform::wayland::WaylandSurface;
#[cfg(target_os = "linux")]
use crate::platform::winit::WinitSurface;
use vello::wgpu::{
    CommandEncoder, Device, Extent3d, SurfaceTexture, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

/// A trait for platform-agnostic surface implementations.
///
/// This trait abstracts over different surface types (winit, Wayland, etc.),
/// allowing the renderer to work with any surface implementation.
pub trait SurfaceTrait {
    /// Get the current surface texture for rendering.
    ///
    /// # Returns
    /// * `Ok(SurfaceTexture)` if texture acquisition succeeded
    /// * `Err(String)` if texture acquisition failed
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String>;

    /// Present the rendered frame to the screen.
    ///
    /// # Returns
    /// * `Ok(())` if presentation succeeded
    /// * `Err(String)` if presentation failed
    fn present(&mut self) -> Result<(), String>;

    /// Resize the surface to the given dimensions.
    ///
    /// # Arguments
    /// * `width` - New surface width in pixels
    /// * `height` - New surface height in pixels
    ///
    /// # Returns
    /// * `Ok(())` if resize succeeded
    /// * `Err(String)` if resize failed
    fn resize(&mut self, width: u32, height: u32) -> Result<(), String>;

    /// Get the surface format.
    ///
    /// # Returns
    /// The texture format used by this surface
    fn format(&self) -> TextureFormat;

    /// Get the current surface size.
    ///
    /// # Returns
    /// A tuple of `(width, height)` in pixels
    fn size(&self) -> (u32, u32);

    /// Check if this surface needs event queue dispatching.
    ///
    /// Some surfaces (like native Wayland) require periodic event queue
    /// dispatching to process window events.
    ///
    /// # Returns
    /// `true` if event dispatching is needed, `false` otherwise
    fn needs_event_dispatch(&self) -> bool;

    /// Dispatch pending events from the surface's event queue.
    ///
    /// This method should be called periodically for surfaces that return
    /// `true` from `needs_event_dispatch()`.
    ///
    /// # Returns
    /// * `Ok(true)` if a redraw is needed after processing events
    /// * `Ok(false)` if no redraw is needed
    /// * `Err(String)` if event dispatching failed
    fn dispatch_events(&mut self) -> Result<bool, String>;

    /// Check if a frame has been presented since the last check.
    /// Returns the frame ready status and resets it to false.
    ///
    /// # Returns
    /// `true` if a frame was presented, `false` otherwise
    fn take_frame_ready(&mut self) -> bool;
}

/// A unified surface that can be either Winit or Wayland.
///
/// This enum wraps different platform surface types to provide a unified interface
/// for rendering, while still allowing platform-specific optimizations.
///
/// This is a rendering abstraction that wraps platform windowing surfaces.
pub enum Surface {
    /// Winit-based surface (works on X11/Wayland via winit abstraction)
    #[cfg(target_os = "linux")]
    Winit(WinitSurface),
    /// Native Wayland surface (direct Wayland protocol)
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    Wayland(WaylandSurface),
}

/// Texture format used for intermediate render targets.
const OFFSCREEN_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// Offscreen texture that serves as the Vello render target.
///
/// This is used by both platform surfaces (WinitSurface and WaylandSurface)
/// for offscreen rendering before blitting to the actual surface.
pub struct OffscreenSurface {
    texture: Texture,
    width: u32,
    height: u32,
}

impl OffscreenSurface {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let clamped_width = width.max(1);
        let clamped_height = height.max(1);
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("nptk-offscreen-surface"),
            size: Extent3d {
                width: clamped_width,
                height: clamped_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Self {
            texture,
            width: clamped_width,
            height: clamped_height,
        }
    }

    /// Get the size of the offscreen surface.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Create a texture view for rendering.
    pub fn create_view(&self) -> TextureView {
        self.texture.create_view(&TextureViewDescriptor::default())
    }
}

// WinitSurface has been moved to platform::winit::WinitSurface

impl SurfaceTrait for Surface {
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String> {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(surface) => surface
                .surface()
                .get_current_texture()
                .map_err(|e| format!("Failed to get surface texture: {:?}", e)),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.get_current_texture(),
        }
    }

    fn present(&mut self) -> Result<(), String> {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(_surface) => {
                // For winit surfaces, present is handled by SurfaceTexture::present()
                // This is a no-op here
                Ok(())
            },
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.present(),
        }
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(surface) => {
                surface.resize(width, height);
                Ok(())
            },
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.resize(width, height),
        }
    }

    fn format(&self) -> TextureFormat {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(surface) => surface.format(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.format(),
        }
    }

    #[allow(unused_variables)]
    fn size(&self) -> (u32, u32) {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(surface) => surface.size(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.size(),
        }
    }

    fn needs_event_dispatch(&self) -> bool {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(_) => false,
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.needs_event_dispatch(),
        }
    }

    fn dispatch_events(&mut self) -> Result<bool, String> {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(_) => Ok(false),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.dispatch_events(),
        }
    }

    fn take_frame_ready(&mut self) -> bool {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(_) => false,
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.take_frame_ready(),
        }
    }
}

impl Surface {
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    pub(crate) fn take_wayland_input_events(&mut self) -> Vec<InputEvent> {
        if let Surface::Wayland(surface) = self {
            return surface.take_pending_input_events();
        }
        Vec::new()
    }

    /// Push an input event to the surface (Wayland only).
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    pub(crate) fn push_input_event(&self, event: InputEvent) {
        if let Surface::Wayland(surface) = self {
            surface.push_input_event(event);
        }
    }

    /// Create a render view for the surface.
    ///
    /// Creates or updates the offscreen render target for the given dimensions.
    pub fn create_render_view(
        &mut self,
        device: &Device,
        width: u32,
        height: u32,
    ) -> Result<TextureView, String> {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(surface) => surface.create_render_view(device, width, height),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(surface) => surface.create_render_view(device, width, height),
        }
    }

    /// Copies the rendered offscreen view into the platform surface texture.
    pub fn blit_render_view(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        source: &TextureView,
        target: &TextureView,
    ) -> Result<(), String> {
        match self {
            #[cfg(target_os = "linux")]
            Surface::Winit(surface) => surface.blit_to_surface(device, encoder, source, target),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(surface) => surface.blit_to_surface(device, encoder, source, target),
        }
    }
}
