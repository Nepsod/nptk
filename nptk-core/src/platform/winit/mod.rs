#![cfg(target_os = "linux")]

//! Winit surface implementation.
//!
//! This module provides a Winit-based surface that works on X11/Wayland via winit abstraction.

use crate::vgi::surface::OffscreenSurface;
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{
    self, CommandEncoder, Device, SurfaceConfiguration, TextureFormat, TextureUsages, TextureView,
};

// OffscreenSurface is imported from VGI

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

    /// Returns a mutable reference to the underlying wgpu surface.
    pub fn surface(&mut self) -> &mut wgpu::Surface<'static> {
        &mut self.surface
    }
}
