//! Renderer abstraction for vector graphics backends.
//!
//! This module provides a unified renderer interface that can work with different
//! rendering backends (Vello, Hybrid, and future backends like tiny skia).

use vello::{RenderParams, Scene as VelloScene};
use vello::wgpu::{Device, Queue, SurfaceTexture};
use super::scene::Scene;
use super::options::RendererOptions;
use super::backend::Backend;

/// A trait for renderer implementations that can render scenes to surfaces.
///
/// This trait allows different backends to provide their own renderer
/// implementations while maintaining a unified API.
pub trait RendererTrait {
    /// Render the scene to a surface texture.
    ///
    /// # Arguments
    /// * `device` - The GPU device
    /// * `queue` - The GPU queue
    /// * `scene` - The scene to render
    /// * `surface_texture` - The surface texture to render to
    /// * `params` - Rendering parameters
    ///
    /// # Returns
    /// * `Ok(())` if rendering succeeded
    /// * `Err(String)` if rendering failed
    fn render_to_surface(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        surface_texture: &SurfaceTexture,
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
    Hybrid(vello_hybrid::Renderer),
}

impl Renderer {
    /// Create a new renderer based on the backend type.
    ///
    /// # Arguments
    /// * `device` - The GPU device
    /// * `backend` - The backend to use
    /// * `options` - Renderer options
    ///
    /// # Returns
    /// * `Ok(Renderer)` if creation succeeded
    /// * `Err(String)` if creation failed
    pub fn new(
        device: &Device,
        backend: Backend,
        options: RendererOptions,
    ) -> Result<Self, String> {
        match backend {
            Backend::Vello => {
                Ok(Renderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create Vello renderer: {:?}", e))?,
                ))
            }
            Backend::Hybrid => {
                // NOTE: vello_hybrid uses wgpu types directly, while vello uses vello::wgpu wrappers.
                // This creates type incompatibility. For now, we'll need to add proper type conversion
                // or use a different approach. Hybrid renderer creation is not yet fully implemented.
                eprintln!("[NPTK] Hybrid renderer creation requires type conversion between vello::wgpu and wgpu types");
                eprintln!("[NPTK] Falling back to Vello renderer for now");
                log::warn!("Hybrid renderer requested but requires type conversion, using Vello");
                Ok(Renderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create renderer: {:?}", e))?,
                ))
            }
            Backend::Custom(_) => {
                // For now, custom backends fall back to Vello
                // In the future, this can be extended with a registry or factory
                eprintln!("[NPTK] Custom backend not yet implemented, using Vello renderer");
                Ok(Renderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create renderer: {:?}", e))?,
                ))
            }
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
    pub fn render_vello_scene_to_surface(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &VelloScene,
        surface_texture: &SurfaceTexture,
        params: &RenderParams,
    ) -> Result<(), String> {
        match self {
            Renderer::Vello(renderer) => {
                renderer
                    .render_to_surface(device, queue, scene, surface_texture, params)
                    .map_err(|e| format!("Vello render error: {:?}", e))?;
                Ok(())
            }
            Renderer::Hybrid(_) => {
                Err("Cannot render Vello scene with Hybrid renderer. Use Scene enum instead.".to_string())
            }
        }
    }

    /// Render the scene to a surface texture.
    ///
    /// This is a convenience method that calls the `RendererTrait::render_to_surface` method.
    pub fn render_to_surface(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        surface_texture: &SurfaceTexture,
        params: &RenderParams,
    ) -> Result<(), String> {
        RendererTrait::render_to_surface(self, device, queue, scene, surface_texture, params)
    }

    /// Update the render target size.
    ///
    /// This is a convenience method that calls the `RendererTrait::update_render_target_size` method.
    pub fn update_render_target_size(&mut self, width: u32, height: u32) {
        RendererTrait::update_render_target_size(self, width, height);
    }
}

impl RendererTrait for Renderer {
    fn render_to_surface(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        surface_texture: &SurfaceTexture,
        params: &RenderParams,
    ) -> Result<(), String> {
        match (self, scene) {
            (Renderer::Vello(renderer), Scene::Vello(vello_scene)) => {
                renderer
                    .render_to_surface(device, queue, vello_scene, surface_texture, params)
                    .map_err(|e| format!("Vello render error: {:?}", e))?;
                Ok(())
            }
            (Renderer::Hybrid(_), Scene::Hybrid(_)) => {
                // Hybrid renderer rendering is not yet fully implemented due to type incompatibilities
                Err("Hybrid renderer rendering requires type conversion, not yet implemented".to_string())
            }
            _ => {
                Err("Renderer and scene backend mismatch".to_string())
            }
        }
    }

    fn update_render_target_size(&mut self, _width: u32, _height: u32) {
        // Hybrid renderer needs scene size, not render target size
        // This is a no-op for now, size is handled by the scene
        match self {
            Renderer::Vello(_) => {},
            Renderer::Hybrid(_) => {},
        }
    }
}

