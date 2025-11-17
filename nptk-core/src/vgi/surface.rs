#![allow(unused_variables)]
//! Platform-agnostic surface abstraction for rendering.
//!
//! This module provides a unified surface interface that can work with different
//! platform backends (Winit, native Wayland, and potentially others).

#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::vgi::wayland_surface::{InputEvent, WaylandSurface};
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{
    self, CommandEncoder, Device, Extent3d, SurfaceConfiguration, SurfaceTexture, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
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
}

/// A unified surface that can be either Winit or Wayland.
///
/// This enum wraps different surface types to provide a unified interface
/// for rendering, while still allowing platform-specific optimizations.
pub enum Surface {
    /// Winit-based surface (works on X11/Wayland via winit abstraction)
    Winit(WinitSurface),
    /// Native Wayland surface (direct Wayland protocol)
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    Wayland(WaylandSurface),
}

/// Texture format used for intermediate render targets.
const OFFSCREEN_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

/// Offscreen texture that serves as the Vello render target.
pub(crate) struct OffscreenSurface {
    texture: Texture,
    width: u32,
    height: u32,
}

impl OffscreenSurface {
    pub(crate) fn new(device: &Device, width: u32, height: u32) -> Self {
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

    pub(crate) fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(crate) fn create_view(&self) -> TextureView {
        self.texture.create_view(&TextureViewDescriptor::default())
    }
}

/// Wrapper around a winit surface that manages offscreen render targets.
pub struct WinitSurface {
    surface: wgpu::Surface<'static>,
    config: Option<SurfaceConfiguration>,
    format: TextureFormat,
    size: (u32, u32),
    offscreen: Option<OffscreenSurface>,
    blitter: Option<TextureBlitter>,
}

impl WinitSurface {
    /// Creates a new wrapper around a winit surface and remembers the initial size.
    pub fn new(surface: wgpu::Surface<'static>, width: u32, height: u32) -> Self {
        Self {
            surface,
            config: None,
            format: TextureFormat::Bgra8Unorm,
            size: (width.max(1), height.max(1)),
            offscreen: None,
            blitter: None,
        }
    }

    /// Configures the swapchain and refreshes the offscreen render target.
    pub fn configure(
        &mut self,
        device: &Device,
        adapter: &wgpu::Adapter,
        width: u32,
        height: u32,
        desired_present_mode: wgpu::PresentMode,
    ) -> Result<(), String> {
        let capabilities = self.surface.get_capabilities(adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| {
                matches!(
                    format,
                    TextureFormat::Bgra8Unorm | TextureFormat::Rgba8Unorm
                )
            })
            .unwrap_or_else(|| capabilities.formats[0]);
        let present_mode = if capabilities.present_modes.contains(&desired_present_mode) {
            desired_present_mode
        } else {
            capabilities
                .present_modes
                .first()
                .copied()
                .unwrap_or(wgpu::PresentMode::Fifo)
        };
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        self.surface.configure(device, &config);
        self.config = Some(config);
        self.format = format;
        self.size = (width.max(1), height.max(1));
        self.offscreen = Some(OffscreenSurface::new(device, self.size.0, self.size.1));
        self.blitter = Some(TextureBlitter::new(device, format));
        Ok(())
    }

    /// Records the latest window size so that the swapchain can be resized lazily.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width.max(1), height.max(1));
    }

    /// Creates (or resizes) the offscreen render target and returns a view for drawing.
    pub fn create_render_view(
        &mut self,
        device: &Device,
        width: u32,
        height: u32,
    ) -> Result<TextureView, String> {
        if self.config.is_none() {
            return Err("Surface not configured".to_string());
        }

        let target_width = width.max(1);
        let target_height = height.max(1);
        self.size = (target_width, target_height);

        if let Some(config) = self.config.as_mut() {
            if config.width != target_width || config.height != target_height {
                config.width = target_width;
                config.height = target_height;
                self.surface.configure(device, config);
                self.offscreen = None;
            }
        }

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

    /// Copies the offscreen render target into the currently acquired surface texture.
    pub fn blit_to_surface(
        &self,
        device: &Device,
        encoder: &mut CommandEncoder,
        source: &TextureView,
        target: &TextureView,
    ) -> Result<(), String> {
        if let Some(blitter) = &self.blitter {
            blitter.copy(device, encoder, source, target);
            Ok(())
        } else {
            Err("Surface not configured".to_string())
        }
    }

    /// Returns the surface format selected during configuration.
    pub fn format(&self) -> TextureFormat {
        self.format
    }

    /// Returns the last known size of the swapchain.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    fn surface(&mut self) -> &mut wgpu::Surface<'static> {
        &mut self.surface
    }
}

impl SurfaceTrait for Surface {
    fn get_current_texture(&mut self) -> Result<SurfaceTexture, String> {
        match self {
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
            Surface::Winit(surface) => surface.format(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.format(),
        }
    }

    #[allow(unused_variables)]
    fn size(&self) -> (u32, u32) {
        match self {
            Surface::Winit(surface) => surface.size(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.size(),
        }
    }

    fn needs_event_dispatch(&self) -> bool {
        match self {
            Surface::Winit(_) => false,
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.needs_event_dispatch(),
        }
    }

    fn dispatch_events(&mut self) -> Result<bool, String> {
        match self {
            Surface::Winit(_) => Ok(false),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(wayland_surface) => wayland_surface.dispatch_events(),
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

    pub fn create_render_view(
        &mut self,
        device: &Device,
        width: u32,
        height: u32,
    ) -> Result<TextureView, String> {
        match self {
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
            Surface::Winit(surface) => surface.blit_to_surface(device, encoder, source, target),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Surface::Wayland(surface) => surface.blit_to_surface(device, encoder, source, target),
        }
    }
}
