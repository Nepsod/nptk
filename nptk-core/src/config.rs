use nalgebra::{Point2, Vector2};
use std::num::NonZeroUsize;
use vello::util::DeviceHandle;
pub use vello::AaConfig;
pub use wgpu_types::PresentMode;
pub use winit::window::{
    BadIcon, Cursor, CursorIcon, CustomCursor, Icon as WindowIcon, WindowButtons, WindowLevel,
};

use nptk_theme::theme::Theme;

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
}

impl<T: Default + Theme> Default for MayConfig<T> {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            render: RenderConfig::default(),
            tasks: None,
            theme: T::default(),
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
#[derive(Clone)]
pub struct RenderConfig {
    /// The rendering backend to use
    ///
    /// Can be configured via `NPTK_RENDERER` environment variable:
    /// - `vello` (default) - Standard Vello GPU renderer
    /// - `hybrid` - Vello Hybrid renderer (currently falls back to Vello)
    pub backend: crate::renderer::RendererBackend,
    /// The antialiasing config
    ///
    /// Can be configured via `NPTK_ANTIALIASING` environment variable:
    /// - `area` (default, fastest) - Area-based antialiasing
    /// - `msaa8` - MSAA 8x (slower but higher quality)
    /// - `msaa16` - MSAA 16x (slowest but best quality)
    pub antialiasing: AaConfig,
    /// If the backend should use the CPU for most drawing operations.
    ///
    /// **NOTE:** The GPU is still used during rasterization.
    ///
    /// Can be enabled via the `NPTK_USE_CPU` environment variable.
    /// Set `NPTK_USE_CPU=true` to enable CPU-based path processing.
    ///
    /// **Note:** This option may not significantly improve performance if the
    /// bottleneck is in GPU rasterization (which still happens) or other Vello bugs.
    pub cpu: bool,
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
    pub device_selector: fn(&Vec<DeviceHandle>) -> &DeviceHandle,
    /// If true, defer system font loading to improve startup performance.
    /// Fonts will be loaded lazily when needed.
    /// Note: Lazy loading may cause text rendering issues if fonts aren't loaded properly.
    pub lazy_font_loading: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        // Check environment variable for CPU rendering
        // Set NPTK_USE_CPU=true, NPTK_USE_CPU=1, or NPTK_USE_CPU=yes to enable CPU-based path processing
        let use_cpu = match std::env::var("NPTK_USE_CPU") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                // Support: true, 1, yes, on, enable
                let enabled = val_lower == "true" 
                    || val_lower == "1" 
                    || val_lower == "yes" 
                    || val_lower == "on"
                    || val_lower == "enable";
                
                if enabled {
                    // Print to stderr to ensure visibility even if logging isn't initialized
                    eprintln!("[NPTK] NPTK_USE_CPU={} detected - enabling CPU path processing", val);
                    eprintln!("[NPTK] Note: GPU is still used for rasterization, only path processing uses CPU");
                    log::info!("NPTK_USE_CPU={} detected - enabling CPU path processing", val);
                    log::info!("Note: GPU is still used for rasterization, only path processing uses CPU");
                } else {
                    log::debug!("NPTK_USE_CPU={} - CPU rendering disabled (expected: true, 1, yes, on, enable)", val);
                }
                enabled
            }
            Err(_) => {
                false
            }
        };
        
        // Check environment variable for antialiasing
        // Options: area (default, fastest), msaa8, msaa16
        let antialiasing = match std::env::var("NPTK_ANTIALIASING") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                let aa = match val_lower.as_str() {
                    "msaa8" => {
                        eprintln!("[NPTK] Using MSAA 8x antialiasing");
                        AaConfig::Msaa8
                    }
                    "msaa16" => {
                        eprintln!("[NPTK] Using MSAA 16x antialiasing");
                        AaConfig::Msaa16
                    }
                    "area" | _ => {
                        if val_lower != "area" {
                            eprintln!("[NPTK] Unknown antialiasing: {}, using Area (fastest)", val);
                        }
                        AaConfig::Area
                    }
                };
                aa
            }
            Err(_) => AaConfig::Area,
        };
        
        // Check environment variable for present mode
        // Options: auto, auto_vsync, fifo, immediate, mailbox
        let present_mode = match std::env::var("NPTK_PRESENT_MODE") {
            Ok(val) => {
                let val_lower = val.to_lowercase();
                let mode = match val_lower.as_str() {
                    "auto_vsync" | "vsync" => {
                        eprintln!("[NPTK] Using VSync present mode");
                        PresentMode::AutoVsync
                    }
                    "fifo" => {
                        eprintln!("[NPTK] Using FIFO present mode");
                        PresentMode::Fifo
                    }
                    "immediate" => {
                        eprintln!("[NPTK] Using Immediate present mode (no VSync, may cause tearing)");
                        PresentMode::Immediate
                    }
                    "mailbox" => {
                        eprintln!("[NPTK] Using Mailbox present mode");
                        PresentMode::Mailbox
                    }
                    "auto" | _ => {
                        if val_lower != "auto" {
                            eprintln!("[NPTK] Unknown present mode: {}, using AutoNoVsync", val);
                        }
                        PresentMode::AutoNoVsync
                    }
                };
                mode
            }
            Err(_) => PresentMode::AutoNoVsync,
        };
        
        // Check environment variable for renderer backend
        let backend = crate::renderer::RendererBackend::from_env();
        
        Self {
            backend,
            antialiasing,
            cpu: use_cpu,
            present_mode,
            init_threads: None,
            device_selector: |devices| devices.first().expect("No devices found"),
            lazy_font_loading: false,
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
