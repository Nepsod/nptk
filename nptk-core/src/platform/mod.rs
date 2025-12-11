//! Platform abstraction for windowing and input.
//!
//! This module provides platform detection and windowing surface implementations
//! for different platforms (Wayland, Winit, etc.).

/// Platform detection and surface creation.
pub mod detection;

#[cfg(all(target_os = "linux", feature = "wayland"))]
pub mod wayland;

#[cfg(target_os = "linux")]
pub mod winit;

#[cfg(all(target_os = "linux", feature = "global-menu"))]
pub mod appmenu;

#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
pub mod xdg_desktop_portal;

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
        detection::detect_platform()
    }
}

// Re-export commonly used types
#[cfg(all(target_os = "linux", feature = "wayland"))]
pub use wayland::{WaylandClient, WaylandQueueHandle, WaylandSurface};
#[cfg(target_os = "linux")]
pub use winit::WinitSurface;

// Re-export detection functions
pub use detection::{create_surface, create_surface_blocking};

// Re-export xdg-portal types
#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
pub use xdg_desktop_portal::{WindowAppearance, XDPEvent, XDPEventSource};

// Re-export MenuInfoStorage for convenience
#[cfg(all(target_os = "linux", feature = "global-menu"))]
pub use appmenu::MenuInfoStorage;
