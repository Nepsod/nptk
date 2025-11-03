//! Renderer abstraction for supporting multiple Vello backends
//!
//! This module provides an abstraction layer over different Vello rendering backends,
//! allowing runtime switching between standard Vello and Vello Hybrid rendering.
//!
//! **Note:** Vello Hybrid has a different API (`vello_hybrid::Scene` vs `vello::Scene`)
//! so full hybrid support would require significant refactoring or Scene type conversion.
//! This module provides a foundation that can be extended when needed.

use vello::{AaSupport, RenderParams, Scene as VelloScene};
use vello::wgpu::{Device, Queue, TextureFormat, SurfaceTexture};
use std::num::NonZeroUsize;

/// The rendering backend to use
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum RendererBackend {
    /// Standard Vello GPU renderer (uses `vello::Scene`)
    Vello,
    /// Vello Hybrid renderer (CPU/GPU hybrid, uses `vello_hybrid::Scene`)
    /// 
    /// **Note:** Hybrid renderer requires using a different Scene type.
    /// Currently not fully implemented - falls back to Vello renderer.
    Hybrid,
}

impl Default for RendererBackend {
    fn default() -> Self {
        RendererBackend::Vello
    }
}

impl RendererBackend {
    /// Parse renderer backend from environment variable `NPTK_RENDERER`
    pub fn from_env() -> Self {
        match std::env::var("NPTK_RENDERER") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                match val_lower.as_str() {
                    "hybrid" => {
                        eprintln!("[NPTK] NPTK_RENDERER=hybrid detected, but hybrid renderer is not yet fully implemented");
                        eprintln!("[NPTK] Falling back to Vello renderer (vello_hybrid requires different Scene API)");
                        log::warn!("Hybrid renderer requested but not fully implemented, using Vello");
                        RendererBackend::Vello // Fall back to Vello for now
                    }
                    "vello" | _ => {
                        if val_lower != "vello" {
                            eprintln!("[NPTK] Unknown renderer: {}, using Vello (standard)", val);
                        }
                        RendererBackend::Vello
                    }
                }
            }
            Err(_) => RendererBackend::default(),
        }
    }
}

/// Unified renderer that can be either Vello or Hybrid
/// 
/// Currently only Vello is fully supported. Hybrid renderer support would require
/// refactoring to use `vello_hybrid::Scene` instead of `vello::Scene`.
pub enum UnifiedRenderer {
    /// Standard Vello renderer
    Vello(vello::Renderer),
}

impl UnifiedRenderer {
    /// Create a new renderer based on the backend type
    pub fn new(
        device: &Device,
        backend: RendererBackend,
        options: RendererOptions,
    ) -> Result<Self, String> {
        match backend {
            RendererBackend::Vello => {
                Ok(UnifiedRenderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create Vello renderer: {:?}", e))?,
                ))
            }
            RendererBackend::Hybrid => {
                // For now, hybrid backend still uses Vello renderer
                // Full hybrid support would require Scene type conversion
                eprintln!("[NPTK] Hybrid backend not yet fully implemented, using Vello renderer");
                Ok(UnifiedRenderer::Vello(
                    vello::Renderer::new(device, options.vello_options())
                        .map_err(|e| format!("Failed to create renderer: {:?}", e))?,
                ))
            }
        }
    }

    /// Render the scene to a surface texture
    pub fn render_to_surface(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &VelloScene,
        surface_texture: &SurfaceTexture,
        params: &RenderParams,
    ) -> Result<(), String> {
        match self {
            UnifiedRenderer::Vello(renderer) => {
                renderer
                    .render_to_surface(device, queue, scene, surface_texture, params)
                    .map_err(|e| format!("Vello render error: {:?}", e))?;
                Ok(())
            }
        }
    }
}

/// Options for creating a renderer
pub struct RendererOptions {
    pub surface_format: Option<TextureFormat>,
    pub use_cpu: bool,
    pub antialiasing_support: AaSupport,
    pub num_init_threads: Option<NonZeroUsize>,
}

impl RendererOptions {
    pub fn vello_options(self) -> vello::RendererOptions {
        vello::RendererOptions {
            surface_format: self.surface_format,
            use_cpu: self.use_cpu,
            antialiasing_support: self.antialiasing_support,
            num_init_threads: self.num_init_threads,
        }
    }
}

