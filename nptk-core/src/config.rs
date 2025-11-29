use crate::vgi::DeviceHandle;
use nalgebra::{Point2, Vector2};
use std::num::NonZeroUsize;
pub use vello::AaConfig;
pub use wgpu_types::PresentMode;
pub use winit::window::{
    BadIcon, Cursor, CursorIcon, CustomCursor, Icon as WindowIcon, WindowButtons, WindowLevel,
};

use nptk_theme::theme::Theme;
use nptk_services::settings::SettingsRegistry;

/// nptk Application Configuration Structure.
#[derive(Clone)]
pub struct MayConfig<T: Theme> {
    /// Window Configuration
    pub window: WindowConfig,
    /// Renderer Configuration.
    pub render: RenderConfig,
    /// Task Runner Configuration. If [None] (default), the task runner won't be enabled.
    pub tasks: Option<TasksConfig>,
    /// Theme of the Application.
    pub theme: T,
    /// Application Settings Registry.
    pub settings: std::sync::Arc<SettingsRegistry>,
}

impl<T: Default + Theme> Default for MayConfig<T> {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            render: RenderConfig::default(),
            tasks: None,
            theme: T::default(),
            settings: std::sync::Arc::new(SettingsRegistry::new().unwrap_or_else(|e| {
                log::error!("Failed to initialize settings registry: {}", e);
                // Return a default registry if loading fails (it has a default impl internally)
                // But SettingsRegistry::new() calls load(), so we might need a fallback constructor
                // For now, let's assume we can construct a default one or handle the error.
                // Since SettingsRegistry doesn't derive Default publicly (it has a new() that returns Result),
                // we might need to expose a safe default or panic.
                // Given this is core config, panicking might be too harsh, but running without settings is also bad.
                // Let's modify SettingsRegistry to derive Default or have a safe default.
                // Wait, I implemented new() -> Result<Self>.
                // I should probably implement Default for SettingsRegistry in nptk-services.
                // For now, I'll use a hack to create an empty one if it fails, or just panic if it's critical.
                // Actually, let's just panic for now as it shouldn't fail unless filesystem is broken.
                panic!("Failed to initialize settings registry: {}", e);
            })),
        }
    }
}

/// Window configuration.
#[derive(Clone)]
pub struct WindowConfig {
    /// The title of the window.
    pub title: String,
    /// The inner size of the window.
    pub size: Vector2<f64>,
    /// The minimum size of the window.
    pub min_size: Option<Vector2<f64>>,
    /// The maximum size of the window.
    pub max_size: Option<Vector2<f64>>,
    /// If the window should be resizeable.
    pub resizable: bool,
    /// If the window should be maximized on startup.
    pub maximized: bool,
    /// The window mode.
    pub mode: WindowMode,
    /// The window level.
    pub level: WindowLevel,
    /// If the window should be visible on startup.
    pub visible: bool,
    /// If the window background should be blurred.
    pub blur: bool,
    /// If the window background should be transparent. May not be compatible on all system.
    pub transparent: bool,
    /// The desired initial position for the window.
    pub position: Option<Point2<f64>>,
    /// If the window should be active/focused on startup.
    pub active: bool,
    /// The enabled window buttons.
    pub buttons: WindowButtons,
    /// If the window should be decorated (have borders).
    pub decorations: bool,
    /// The resize increments of the window. Not supported everywhere.
    pub resize_increments: Option<Vector2<f64>>,
    /// Prevents window capturing by some apps (not all though).
    pub content_protected: bool,
    /// The window icon.
    pub icon: Option<WindowIcon>,
    /// The window cursor.
    pub cursor: Cursor,
    /// If the window should exit/close on close request (pressing the close window button).
    pub close_on_request: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "New App".to_string(),
            size: Vector2::new(800.0, 600.0),
            min_size: None,
            max_size: None,
            resizable: true,
            maximized: false,
            mode: WindowMode::default(),
            level: Default::default(),
            visible: true,
            blur: false,
            transparent: false,
            position: None,
            active: true,
            buttons: WindowButtons::all(),
            decorations: true,
            resize_increments: None,
            content_protected: false,
            icon: None,
            cursor: Cursor::default(),
            close_on_request: true,
        }
    }
}

/// Renderer configuration.
///
/// **Performance Note:** If you experience lag/performance issues, this may be due to
/// known bugs in the Vello rendering backend. Various environment variables can be used
/// to experiment with different rendering settings, though they may not resolve
/// fundamental issues in Vello itself.
///
/// **Renderer Backend:** Can be configured via `NPTK_RENDERER` environment variable:
/// - `vello` (default) - Standard Vello GPU renderer
/// - `hybrid` - Vello Hybrid renderer (currently falls back to Vello; `vello_hybrid` requires different Scene API)
///
/// **Windowing Platform:** Can be configured via `NPTK_PLATFORM` environment variable:
/// - `winit` (X11 only) - Use winit-based windowing (works on X11 via winit abstraction)
/// - `wayland` - Use native Wayland windowing (direct Wayland protocol, Linux only)
#[derive(Clone)]
pub struct RenderConfig {
    /// The rendering backend to use
    ///
    /// Can be configured via `NPTK_RENDERER` environment variable:
    /// - `vello` (default) - Standard Vello GPU renderer
    /// - `hybrid` - Vello Hybrid renderer (currently falls back to Vello)
    ///
    /// **Note:** Windowing platform selection (winit vs native Wayland) is controlled
    /// separately via the `NPTK_PLATFORM` environment variable.
    pub backend: crate::vgi::Backend,
    /// The antialiasing config
    ///
    /// Can be configured via `NPTK_ANTIALIASING` environment variable:
    /// - `area` (default, fastest) - Area-based antialiasing
    /// - `msaa8` - MSAA 8x (slower but higher quality)
    /// - `msaa16` - MSAA 16x (slowest but best quality)
    pub antialiasing: AaConfig,

    /// The presentation mode of the window/surface.
    ///
    /// Can be configured via `NPTK_PRESENT_MODE` environment variable:
    /// - `auto` (default, no VSync) - Auto-detect, no VSync
    /// - `vsync` or `auto_vsync` - Enable VSync (may smooth out but limit FPS)
    /// - `immediate` - No sync (lowest latency, may cause tearing)
    /// - `fifo` - FIFO queue (VSync-like)
    /// - `mailbox` - Mailbox mode (triple buffering)
    ///
    /// **Note:** `immediate` mode typically provides the lowest latency.
    pub present_mode: PresentMode,
    /// The number of threads to use for initialization in [vello].
    pub init_threads: Option<NonZeroUsize>,
    /// The selector function to determine which device to use for rendering. Defaults to using the first device found.
    pub device_selector: fn(&[DeviceHandle]) -> &DeviceHandle,

}

impl Default for RenderConfig {
    fn default() -> Self {


        // Check environment variable for antialiasing
        // Options: area (default, fastest), msaa8, msaa16
        let antialiasing = match std::env::var("NPTK_ANTIALIASING") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                let aa = match val_lower.as_str() {
                    "msaa8" => {
                        log::info!("Using MSAA 8x antialiasing");
                        AaConfig::Msaa8
                    },
                    "msaa16" => {
                        log::info!("Using MSAA 16x antialiasing");
                        AaConfig::Msaa16
                    },
                    "area" | _ => {
                        if val_lower != "area" {
                            log::warn!(
                                "Unknown antialiasing '{}'; defaulting to Area (fastest)",
                                val
                            );
                        }
                        AaConfig::Area
                    },
                };
                aa
            },
            Err(_) => AaConfig::Area,
        };

        // Check environment variable for present mode
        // Options: auto, auto_vsync, fifo, immediate, mailbox
        let present_mode = match std::env::var("NPTK_PRESENT_MODE") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                let mode = match val_lower.as_str() {
                    "auto_vsync" | "vsync" => {
                        log::info!("Using VSync present mode");
                        PresentMode::AutoVsync
                    },
                    "fifo" => {
                        log::info!("Using FIFO present mode");
                        PresentMode::Fifo
                    },
                    "immediate" => {
                        log::info!("Using Immediate present mode (no VSync, may cause tearing)");
                        PresentMode::Immediate
                    },
                    "mailbox" => {
                        log::info!("Using Mailbox present mode");
                        PresentMode::Mailbox
                    },
                    "auto" | _ => {
                        if val_lower != "auto" {
                            log::warn!("Unknown present mode '{}'; using AutoNoVsync", val);
                        }
                        PresentMode::AutoNoVsync
                    },
                };
                mode
            },
            Err(_) => PresentMode::AutoNoVsync,
        };

        // Check environment variable for renderer backend
        let backend = crate::vgi::Backend::from_env();

        Self {
            backend,
            antialiasing,

            present_mode,
            init_threads: None,
            device_selector: |devices| devices.first().expect("No devices found"),

        }
    }
}

/// The window mode.
#[derive(Clone, Debug, Default)]
pub enum WindowMode {
    /// The default windowed mode.
    #[default]
    Windowed,
    /// Size the window to fill the screen and remove borders. This is more modern, than default Fullscreen.
    Borderless,
    /// Legacy Fullscreen mode.
    Fullscreen,
}

/// Configuration structure for the integrated [TaskRunner](crate::tasks::TaskRunner).
///
/// The task runner isn't used by nptk internally, but can be used to spawn asynchronous tasks and integrate them with the UI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TasksConfig {
    /// The stack size of each thread of the task runner thread pool. Defaults to 1 MB.
    pub stack_size: usize,
    /// The amount of worker threads of the task runner thread pool. Defaults to half of the available threads.
    pub workers: NonZeroUsize,
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            stack_size: 1024 * 1024, // 1 MB
            workers: NonZeroUsize::new(
                std::thread::available_parallelism()
                    .expect("Failed to get available threads")
                    .get()
                    / 2,
            )
            .unwrap(),
        }
    }
}
