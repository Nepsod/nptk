//! Renderer options and configuration.
//!
//! This module provides configuration structures for creating renderers
//! with different backends.

use vello::AaSupport;
use vello::wgpu::TextureFormat;
use std::num::NonZeroUsize;
use wgpu::TextureFormat as WgpuTextureFormat;

/// Options for creating a renderer.
///
/// This structure contains all the configuration needed to create a renderer
/// for a specific backend. Different backends may use different subsets of
/// these options.
pub struct RendererOptions {
    /// Surface format for rendering (used by Vello and Hybrid backends)
    pub surface_format: Option<TextureFormat>,
    /// Whether to use CPU for path processing (Vello-specific)
    pub use_cpu: bool,
    /// Antialiasing support configuration
    pub antialiasing_support: AaSupport,
    /// Number of initialization threads (optional)
    pub num_init_threads: Option<NonZeroUsize>,
}

impl RendererOptions {
    /// Convert these options to Vello-specific renderer options.
    ///
    /// This method is used when creating a Vello renderer.
    pub fn vello_options(self) -> vello::RendererOptions {
        vello::RendererOptions {
            surface_format: self.surface_format,
            use_cpu: self.use_cpu,
            antialiasing_support: self.antialiasing_support,
            num_init_threads: self.num_init_threads,
        }
    }

    /// Convert these options to Hybrid-specific renderer options.
    ///
    /// This method is used when creating a Hybrid renderer.
    /// Returns None if surface_format is not available.
    /// 
    /// Note: This requires converting vello::wgpu::TextureFormat to wgpu::TextureFormat.
    /// Since they're different types but represent the same enum, we use unsafe conversion
    /// as a workaround. This is safe because both types have the same memory layout.
    pub fn hybrid_render_target_config(&self, width: u32, height: u32) -> Option<vello_hybrid::RenderTargetConfig> {
        self.surface_format.map(|format| {
            // Convert vello::wgpu::TextureFormat to wgpu::TextureFormat
            // Both are enums with identical memory layout, so we can use unsafe conversion
            // This is safe because the types are structurally identical
            let wgpu_format = unsafe {
                std::mem::transmute::<vello::wgpu::TextureFormat, WgpuTextureFormat>(format)
            };
            
            vello_hybrid::RenderTargetConfig {
                format: wgpu_format,
                width,
                height,
            }
        })
    }

    // Future: Add methods for other backends like:
    // pub fn tiny_skia_options(self) -> tiny_skia::RendererOptions { ... }
}

