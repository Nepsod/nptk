//! Platform detection and surface creation utilities.
//!
//! This module provides functions to detect the current platform (Winit vs Wayland)
//! and create appropriate surfaces based on the platform.

use crate::vgi::surface::{Surface, WinitSurface};
#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::vgi::wayland_surface::WaylandSurface;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// * `Platform::Wayland` if `NPTK_PLATFORM` is set to "wayland" (native Wayland windowing)
    /// * `Platform::Wayland` if `WAYLAND_DISPLAY` is set (indicates Wayland session, auto-detected)
    /// * `Platform::Winit` if `NPTK_PLATFORM` is set to "winit" (winit-based windowing)
    /// * `Platform::Winit` otherwise (default, uses winit abstraction, works on X11/XWayland)
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            #[cfg(feature = "wayland")]
            {
                // Check if platform is explicitly requested via NPTK_PLATFORM
                if let Ok(val) = std::env::var("NPTK_PLATFORM") {
                    let val_lower = val.to_lowercase();
                    match val_lower.as_str() {
                        "wayland" => {
                            log::debug!("Native Wayland windowing requested via NPTK_PLATFORM=wayland");
                            return Platform::Wayland;
                        },
                        "winit" => {
                            log::debug!("Winit windowing requested via NPTK_PLATFORM=winit");
                            return Platform::Winit;
                        },
                        _ => {
                            log::warn!(
                                "Unknown NPTK_PLATFORM value '{}'; defaulting to Winit",
                                val
                            );
                        },
                    }
                }

                // Check if we're in a Wayland session (auto-detect)
                if std::env::var("WAYLAND_DISPLAY").is_ok() {
                    // Only log this message once to avoid spam
                    static LOGGED: AtomicBool = AtomicBool::new(false);
                    if !LOGGED.swap(true, Ordering::Relaxed) {
                        log::info!("WAYLAND_DISPLAY detected, using native Wayland windowing");
                    }
                    return Platform::Wayland;
                }
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
    gpu_context: Option<&crate::vgi::GpuContext>,
) -> Result<Surface, String> {
    match platform {
        Platform::Winit => {
            let window = window.ok_or_else(|| "Window required for Winit platform".to_string())?;
            let gpu_context =
                gpu_context.ok_or_else(|| "GpuContext required for Winit platform".to_string())?;

            // Create surface using GpuContext's Instance
            // Note: winit 0.30 with X11-only doesn't implement HasWindowHandle/HasDisplayHandle
            // We need to use create_surface_unsafe with raw window handles from raw-window-handle
            let instance = gpu_context.instance();
            
            // Get raw window handles using raw-window-handle crate
            use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
            let window_handle = (*window).window_handle()
                .map_err(|e| format!("Failed to get window handle: {:?}", e))?;
            let display_handle = (*window).display_handle()
                .map_err(|e| format!("Failed to get display handle: {:?}", e))?;
            
            let raw_window = window_handle.as_raw();
            let raw_display = display_handle.as_raw();
            
            let target = vello::wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: raw_display,
                raw_window_handle: raw_window,
            };
            
            let surface = unsafe { instance.create_surface_unsafe(target) }
                .map_err(|e| format!("Failed to create winit surface: {:?}", e))?;

            Ok(Surface::Winit(WinitSurface::new(surface, width, height)))
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
            // Note: winit 0.30 with X11-only doesn't implement HasWindowHandle/HasDisplayHandle
            // We need to use create_surface_unsafe with raw window handles from raw-window-handle
            let instance = gpu_context.instance();
            
            // Get raw window handles using raw-window-handle crate
            use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
            let window_handle = (*window).window_handle()
                .map_err(|e| format!("Failed to get window handle: {:?}", e))?;
            let display_handle = (*window).display_handle()
                .map_err(|e| format!("Failed to get display handle: {:?}", e))?;
            
            let raw_window = window_handle.as_raw();
            let raw_display = display_handle.as_raw();
            
            let target = vello::wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: raw_display,
                raw_window_handle: raw_window,
            };
            
            let surface = unsafe { instance.create_surface_unsafe(target) }
                .map_err(|e| format!("Failed to create winit surface: {:?}", e))?;

            Ok(Surface::Winit(WinitSurface::new(surface, width, height)))
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
