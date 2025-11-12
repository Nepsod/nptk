//! Backend selection and configuration for vector graphics rendering.
//!
//! This module provides types and utilities for selecting and configuring
//! different rendering backends (Vello, Hybrid, and future backends).

/// The rendering backend to use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Backend {
    /// Standard Vello GPU renderer (uses `vello::Scene`)
    Vello,
    /// Vello Hybrid renderer (CPU/GPU hybrid, uses `vello_hybrid::Scene`)
    ///
    /// **Note:** Hybrid renderer requires using a different Scene type.
    /// Currently not fully implemented - falls back to Vello renderer.
    Hybrid,
    /// Custom backend (for future extensibility, e.g., tiny skia).
    ///
    /// The string identifier can be used to look up backend implementations
    /// in a registry or factory system.
    Custom(String),
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Vello
    }
}

impl Backend {
    /// Parse renderer backend from environment variable `NPTK_RENDERER`.
    ///
    /// Valid values:
    /// - `vello` (default) - Standard Vello GPU renderer
    /// - `hybrid` - Vello Hybrid renderer (CPU/GPU hybrid)
    /// - `wayland` - Special value: uses native Wayland windowing (rendering backend is still Vello)
    /// - Any other value will default to Vello
    ///
    /// **Note:** `wayland` is handled by `Platform::detect()` for windowing platform selection.
    /// This function only handles rendering backend selection, so `wayland` is treated as unknown.
    pub fn from_env() -> Self {
        match std::env::var("NPTK_RENDERER") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                match val_lower.as_str() {
                    "hybrid" => {
                        log::info!("NPTK_RENDERER=hybrid detected; using Vello Hybrid renderer");
                        Backend::Hybrid
                    },
                    "wayland" => {
                        // wayland is handled by Platform::detect() for windowing
                        // For rendering backend, default to Vello
                        log::info!(
                            "NPTK_RENDERER=wayland sets windowing platform, using Vello renderer"
                        );
                        Backend::Vello
                    },
                    "vello" | "" => Backend::Vello,
                    custom => {
                        log::warn!(
                            "Unknown renderer '{}'; falling back to Vello (standard)",
                            custom
                        );
                        log::debug!("Use Backend::Custom(\"{}\") for custom backends", custom);
                        Backend::Vello
                    },
                }
            },
            Err(_) => Backend::default(),
        }
    }
}
