//! Platform detection and surface creation utilities.
//!
//! This module provides functions to detect the current platform (Winit vs Wayland)
//! and create appropriate surfaces based on the platform.

use crate::vgi::surface::Surface;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::vgi::wayland_surface::WaylandSurface;
use std::sync::Arc;

/// Platform type for surface creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Use winit-based surface (works on X11/Wayland via winit abstraction)
    Winit,
    /// Use native Wayland surface (direct Wayland protocol)
    #[cfg(target_os = "linux")]
    Wayland,
}

impl Platform {
    /// Detect the platform to use based on environment variables and system state.
    ///
    /// # Returns
    /// * `Platform::Wayland` if `NPTK_RENDERER` is set to "wayland" (native Wayland windowing)
    /// * `Platform::Wayland` if `WAYLAND_DISPLAY` is set (indicates Wayland session, auto-detected)
    /// * `Platform::Winit` otherwise (uses winit abstraction, works on X11/Wayland/X11)
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            #[cfg(feature = "wayland")]
            {
                // Check if native Wayland is explicitly requested via NPTK_RENDERER=wayland
                if let Ok(val) = std::env::var("NPTK_RENDERER") {
                    let val_lower = val.to_lowercase();
                    if val_lower == "wayland" {
                        log::info!("Native Wayland windowing requested via NPTK_RENDERER=wayland");
                        return Platform::Wayland;
                    }
                }
            }

            // Check if we're in a Wayland session (auto-detect)
            // if std::env::var("WAYLAND_DISPLAY").is_ok() {
            //     eprintln!("[NPTK] WAYLAND_DISPLAY detected, using native Wayland windowing");
            //     log::info!("Wayland session detected, using native Wayland windowing");
            //     return Platform::Wayland;
            // }
        }

        Platform::Winit
    }
}

/// Create a surface based on the platform.
///
/// # Arguments
/// * `platform` - The platform to use
/// * `window` - Winit window (required for Winit platform, ignored for Wayland)
/// * `width` - Surface width in pixels
/// * `height` - Surface height in pixels
/// * `title` - Window title (used for Wayland platform)
/// * `render_ctx` - Render context (required for Winit platform, ignored for Wayland)
///
/// # Returns
/// * `Ok(Surface)` if creation succeeded
/// * `Err(String)` if creation failed
#[cfg(target_os = "linux")]
pub async fn create_surface(
    platform: Platform,
    window: Option<Arc<winit::window::Window>>,
    width: u32,
    height: u32,
    title: &str,
    gpu_context: Option<&crate::vgi::GpuContext>,
) -> Result<Surface, String> {
    match platform {
        Platform::Winit => {
            let window = window.ok_or_else(|| "Window required for Winit platform".to_string())?;
            let gpu_context =
                gpu_context.ok_or_else(|| "GpuContext required for Winit platform".to_string())?;

            // Create surface using GpuContext's Instance
            let instance = gpu_context.instance();
            let surface = instance
                .create_surface(window.clone())
                .map_err(|e| format!("Failed to create winit surface: {:?}", e))?;

            Ok(Surface::Winit(surface))
        },
        Platform::Wayland => {
            #[cfg(feature = "wayland")]
            {
                let gpu_context = gpu_context
                    .ok_or_else(|| "GpuContext required for Wayland platform".to_string())?;
                let wayland_surface = WaylandSurface::new(width, height, title, gpu_context)?;
                Ok(Surface::Wayland(wayland_surface))
            }
            #[cfg(not(feature = "wayland"))]
            {
                Err("Wayland feature is disabled".to_string())
            }
        },
    }
}

/// Create a surface based on the platform (non-async version for convenience).
///
/// This is a blocking wrapper around the async `create_surface` function.
#[cfg(target_os = "linux")]
pub fn create_surface_blocking(
    platform: Platform,
    window: Option<Arc<winit::window::Window>>,
    width: u32,
    height: u32,
    title: &str,
    gpu_context: Option<&crate::vgi::GpuContext>,
) -> Result<Surface, String> {
    crate::tasks::block_on(create_surface(
        platform,
        window,
        width,
        height,
        title,
        gpu_context,
    ))
}

#[cfg(not(target_os = "linux"))]
pub async fn create_surface(
    platform: Platform,
    window: Option<Arc<winit::window::Window>>,
    width: u32,
    height: u32,
    _title: &str,
    gpu_context: Option<&crate::vgi::GpuContext>,
) -> Result<Surface, String> {
    match platform {
        Platform::Winit => {
            let window = window.ok_or_else(|| "Window required for Winit platform".to_string())?;
            let gpu_context =
                gpu_context.ok_or_else(|| "GpuContext required for Winit platform".to_string())?;

            // Create surface using GpuContext's Instance
            let instance = gpu_context.instance();
            let surface = instance
                .create_surface(window.clone())
                .map_err(|e| format!("Failed to create winit surface: {:?}", e))?;

            Ok(Surface::Winit(surface))
        },
    }
}

#[cfg(not(target_os = "linux"))]
pub fn create_surface_blocking(
    platform: Platform,
    window: Option<Arc<winit::window::Window>>,
    width: u32,
    height: u32,
    _title: &str,
    gpu_context: Option<&crate::vgi::GpuContext>,
) -> Result<Surface, String> {
    crate::tasks::block_on(create_surface(
        platform,
        window,
        width,
        height,
        "",
        gpu_context,
    ))
}
