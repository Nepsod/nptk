//! Platform-agnostic surface abstraction for rendering.
//!
//! This module provides a unified surface interface that can work with different
//! platform backends (Winit, native Wayland, and potentially others).

use vello::wgpu::{SurfaceTexture, TextureFormat};

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
}

/// A unified surface that can be either Winit or Wayland.
///
/// This enum wraps different surface types to provide a unified interface
/// for rendering, while still allowing platform-specific optimizations.
pub enum Surface {
    /// Winit-based surface (works on X11/Wayland via winit abstraction)
    Winit(vello::wgpu::Surface<'static>),
    /// Native Wayland surface (direct Wayland protocol)
    #[cfg(target_os = "linux")]
    Wayland(crate::vgi::wayland_surface::WaylandSurface),
}

impl SurfaceTrait for Surface {
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String> {
        match self {
            Surface::Winit(surface) => {
                surface
                    .get_current_texture()
                    .map_err(|e| format!("Failed to get surface texture: {:?}", e))
            }
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.get_current_texture(),
        }
    }

    fn present(&mut self) -> Result<(), String> {
        match self {
            Surface::Winit(_surface) => {
                // For winit surfaces, present is handled by SurfaceTexture::present()
                // This is a no-op here
                Ok(())
            }
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.present(),
        }
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        match self {
            Surface::Winit(_surface) => {
                // Winit surfaces are resized via RenderContext::resize_surface()
                // This is handled externally
                Ok(())
            }
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.resize(width, height),
        }
    }

    fn format(&self) -> TextureFormat {
        match self {
            Surface::Winit(_surface) => {
                // TODO: Get format from surface/adapter properly
                // For now, return default format
                // The format should be obtained from RenderSurface when it's created
                TextureFormat::Bgra8Unorm
            }
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.format(),
        }
    }

    fn size(&self) -> (u32, u32) {
        match self {
            Surface::Winit(surface) => {
                // Winit surface size must be obtained from the window
                // This is a limitation - we need access to the window to get size
                // For now, return 0x0 and let the caller handle it
                // The caller should use window.inner_size() instead
                (0, 0)
            }
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.size(),
        }
    }

    fn needs_event_dispatch(&self) -> bool {
        match self {
            Surface::Winit(_) => false,
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.needs_event_dispatch(),
        }
    }

    fn dispatch_events(&mut self) -> Result<bool, String> {
        match self {
            Surface::Winit(_) => Ok(false),
            #[cfg(target_os = "linux")]
            Surface::Wayland(wayland_surface) => wayland_surface.dispatch_events(),
        }
    }
}

