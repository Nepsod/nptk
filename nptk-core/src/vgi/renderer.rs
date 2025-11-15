//! Renderer abstraction for vector graphics backends.
//!
//! This module provides a unified renderer interface that can work with different
//! rendering backends (Vello, Hybrid, and future backends like tiny skia).

use super::backend::Backend;
use super::options::RendererOptions;
use super::scene::Scene;
use vello::wgpu::{Device, Queue, TextureView};
use vello::{RenderParams, Scene as VelloScene};
#[cfg(feature = "vello-hybrid")]
use vello_hybrid::Renderer as HybridRenderer;

/// A trait for renderer implementations that can render scenes to surfaces.
///
/// This trait allows different backends to provide their own renderer
/// implementations while maintaining a unified API.
pub trait RendererTrait {
    /// Render the scene to a texture view.
    ///
    /// # Arguments
    /// * `device` - The GPU device
    /// * `queue` - The GPU queue
    /// * `scene` - The scene to render
    /// * `texture_view` - The texture view to render to
    /// * `params` - Rendering parameters
    ///
    /// # Returns
    /// * `Ok(())` if rendering succeeded
    /// * `Err(String)` if rendering failed
    fn render_to_view(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        texture_view: &TextureView,
        params: &RenderParams,
    ) -> Result<(), String>;

    /// Update the render target size.
    ///
    /// This method is called when the window is resized or the render target
    /// size changes. Some backends may need this information.
    fn update_render_target_size(&mut self, _width: u32, _height: u32) {
        // Default implementation is a no-op
        // Backends can override if needed
    }
}

/// A unified renderer that can be either Vello or Hybrid.
///
/// This enum wraps different renderer types to provide a unified interface
/// for rendering, while still allowing backend-specific optimizations.
pub enum Renderer {
    /// Standard Vello renderer
    Vello(vello::Renderer),
    /// Vello Hybrid renderer (CPU/GPU hybrid)
    #[cfg(feature = "vello-hybrid")]
    Hybrid(HybridRenderer),
}

impl Renderer {
    /// Create a new renderer based on the backend type.
    ///
    /// # Arguments
    /// * `device` - The GPU device
    /// * `backend` - The backend to use
    /// * `options` - Renderer options
    /// * `width` - Render target width (used for Hybrid backend)
    /// * `height` - Render target height (used for Hybrid backend)
    ///
    /// # Returns
    /// * `Ok(Renderer)` if creation succeeded
    /// * `Err(String)` if creation failed
    pub fn new(
        device: &Device,
        backend: Backend,
        options: RendererOptions,
        _width: u32,
        _height: u32,
    ) -> Result<Self, String> {
        match backend {
            Backend::Vello => Ok(Renderer::Vello(
                vello::Renderer::new(device, options.vello_options())
                    .map_err(|e| format!("Failed to create Vello renderer: {:?}", e))?,
            )),
            Backend::Hybrid => {
                #[cfg(feature = "vello-hybrid")]
                {
                    // CRITICAL: vello_hybrid uses wgpu 26.0.1, while vello uses wgpu 23.0.1.
                    // These are incompatible versions and cannot be safely converted.
                    // For now, Hybrid backend is disabled until we can resolve the version conflict.
                    log::error!("Hybrid renderer requested but unavailable due to wgpu version conflict (vello=23.0.1, vello_hybrid=26.0.1)");
                    log::warn!("Falling back to Vello renderer");
                }
                #[cfg(not(feature = "vello-hybrid"))]
                {
                    log::warn!("Hybrid renderer requested but the 'vello-hybrid' feature is disabled; falling back to Vello renderer");
                }
                Ok(Renderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create renderer: {:?}", e))?,
                ))
            },
            Backend::Custom(_) => {
                // For now, custom backends fall back to Vello
                // In the future, this can be extended with a registry or factory
                log::info!("Custom backend not yet implemented; using Vello renderer");
                Ok(Renderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create renderer: {:?}", e))?,
                ))
            },
        }
    }

    /// Render a Vello scene (legacy method for compatibility).
    ///
    /// This method is provided for backward compatibility with code that
    /// uses `vello::Scene` directly.
    ///
    /// # Arguments
    /// * `device` - The GPU device
    /// * `queue` - The GPU queue
    /// * `scene` - The Vello scene to render
    /// * `surface_texture` - The surface texture to render to
    /// * `params` - Rendering parameters
    ///
    /// # Returns
    /// * `Ok(())` if rendering succeeded
    /// * `Err(String)` if rendering failed or renderer is not Vello
    pub fn render_vello_scene_to_view(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &VelloScene,
        texture_view: &TextureView,
        params: &RenderParams,
    ) -> Result<(), String> {
        match self {
            Renderer::Vello(renderer) => {
                renderer
                    .render_to_texture(device, queue, scene, texture_view, params)
                    .map_err(|e| format!("Vello render error: {:?}", e))?;
                Ok(())
            },
            #[cfg(feature = "vello-hybrid")]
            Renderer::Hybrid(_) => Err(
                "Cannot render Vello scene with Hybrid renderer. Use Scene enum instead."
                    .to_string(),
            ),
        }
    }

    /// Render the scene to a texture view.
    ///
    /// This is a convenience method that calls the `RendererTrait::render_to_view` method.
    pub fn render_to_view(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        texture_view: &TextureView,
        params: &RenderParams,
    ) -> Result<(), String> {
        RendererTrait::render_to_view(self, device, queue, scene, texture_view, params)
    }

    /// Update the render target size.
    ///
    /// This is a convenience method that calls the `RendererTrait::update_render_target_size` method.
    pub fn update_render_target_size(&mut self, width: u32, height: u32) {
        RendererTrait::update_render_target_size(self, width, height);
    }
}

impl RendererTrait for Renderer {
    fn render_to_view(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        texture_view: &TextureView,
        params: &RenderParams,
    ) -> Result<(), String> {
        #[cfg(feature = "vello-hybrid")]
        {
            match (self, scene) {
                (Renderer::Vello(renderer), Scene::Vello(vello_scene)) => {
                    renderer
                        .render_to_texture(device, queue, vello_scene, texture_view, params)
                        .map_err(|e| format!("Vello render error: {:?}", e))?;
                    Ok(())
                },
                (Renderer::Hybrid(_), Scene::Hybrid(_)) => {
                    // Hybrid renderer is disabled due to wgpu version conflict
                    Err("Hybrid renderer is not available due to wgpu version conflict between vello (23.0.1) and vello_hybrid (26.0.1)".to_string())
                },
                _ => Err("Renderer and scene backend mismatch".to_string()),
            }
        }
        #[cfg(not(feature = "vello-hybrid"))]
        {
            #[allow(irrefutable_let_patterns)]
            if let (Renderer::Vello(renderer), Scene::Vello(vello_scene)) = (self, scene) {
                renderer
                    .render_to_texture(device, queue, vello_scene, texture_view, params)
                    .map_err(|e| format!("Vello render error: {:?}", e))?;
                Ok(())
            } else {
                Err("Renderer and scene backend mismatch".to_string())
            }
        }
    }

    fn update_render_target_size(&mut self, _width: u32, _height: u32) {
        // Hybrid renderer needs scene size, not render target size
        // This is a no-op for now, size is handled by the scene
        match self {
            Renderer::Vello(_) => {},
            #[cfg(feature = "vello-hybrid")]
            Renderer::Hybrid(_) => {},
        }
    }
}
