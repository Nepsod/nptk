//! Platform detection and surface creation utilities.
//!
//! This module provides functions to detect the current platform (Winit vs Wayland)
//! and create appropriate surfaces based on the platform.

use crate::vgi::surface::Surface;
#[cfg(target_os = "linux")]
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
    /// * `Platform::Wayland` if `NPTK_USE_NATIVE_WAYLAND` is set to "true"/"1" and Wayland is available
    /// * `Platform::Wayland` if `WAYLAND_DISPLAY` is set (indicates Wayland session)
    /// * `Platform::Winit` otherwise
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            // Check if native Wayland is explicitly requested
            if let Ok(val) = std::env::var("NPTK_USE_NATIVE_WAYLAND") {
                let val_lower = val.to_lowercase();
                if matches!(val_lower.as_str(), "true" | "1" | "yes" | "on" | "enable") {
                    eprintln!("[NPTK] NPTK_USE_NATIVE_WAYLAND detected, attempting native Wayland");
                    log::info!("Native Wayland requested via NPTK_USE_NATIVE_WAYLAND");
                    return Platform::Wayland;
                }
            }

            // Check if we're in a Wayland session
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                eprintln!("[NPTK] WAYLAND_DISPLAY detected, using native Wayland");
                log::info!("Wayland session detected, using native Wayland");
                return Platform::Wayland;
            }
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
    render_ctx: Option<&mut vello::util::RenderContext>,
) -> Result<Surface, String> {
    match platform {
        Platform::Winit => {
            let window = window.ok_or_else(|| "Window required for Winit platform".to_string())?;
            let render_ctx = render_ctx.ok_or_else(|| "RenderContext required for Winit platform".to_string())?;
            
            let render_surface = render_ctx
                .create_surface(
                    window,
                    width,
                    height,
                    vello::wgpu::PresentMode::AutoVsync,
                )
                .await
                .map_err(|e| format!("Failed to create winit surface: {:?}", e))?;
            
            // Extract the surface - RenderSurface contains Surface<'static>
            Ok(Surface::Winit(render_surface.surface))
        }
        Platform::Wayland => {
            let wayland_surface = WaylandSurface::new(width, height, title)?;
            Ok(Surface::Wayland(wayland_surface))
        }
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
    render_ctx: Option<&mut vello::util::RenderContext>,
) -> Result<Surface, String> {
    crate::tasks::block_on(create_surface(platform, window, width, height, title, render_ctx))
}

#[cfg(not(target_os = "linux"))]
pub async fn create_surface(
    platform: Platform,
    window: Option<Arc<winit::window::Window>>,
    width: u32,
    height: u32,
    _title: &str,
    render_ctx: Option<&mut vello::util::RenderContext>,
) -> Result<Surface, String> {
    match platform {
        Platform::Winit => {
            let window = window.ok_or_else(|| "Window required for Winit platform".to_string())?;
            let render_ctx = render_ctx.ok_or_else(|| "RenderContext required for Winit platform".to_string())?;
            
            let render_surface = render_ctx
                .create_surface(
                    window,
                    width,
                    height,
                    vello::wgpu::PresentMode::AutoVsync,
                )
                .await
                .map_err(|e| format!("Failed to create winit surface: {:?}", e))?;
            
            Ok(Surface::Winit(render_surface.surface))
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn create_surface_blocking(
    platform: Platform,
    window: Option<Arc<winit::window::Window>>,
    width: u32,
    height: u32,
    _title: &str,
    render_ctx: Option<&mut vello::util::RenderContext>,
) -> Result<Surface, String> {
    crate::tasks::block_on(create_surface(platform, window, width, height, "", render_ctx))
}

