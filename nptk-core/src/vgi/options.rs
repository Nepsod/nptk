//! Renderer options and configuration.
//!
//! This module provides configuration structures for creating renderers
//! with different backends.

use std::num::NonZeroUsize;
use vello::AaSupport;

/// Options for creating a renderer.
///
/// This structure contains all the configuration needed to create a renderer
/// for a specific backend. Different backends may use different subsets of
/// these options.
pub struct RendererOptions {
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
            use_cpu: self.use_cpu,
            antialiasing_support: self.antialiasing_support,
            num_init_threads: self.num_init_threads,
            pipeline_cache: None,
        }
    }
}
