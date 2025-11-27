//! XDG Desktop Portal integration for system settings and appearance.
//!
//! This module uses the [ashpd] crate to monitor system settings like color scheme,
//! cursor theme, and cursor size through the XDG Desktop Portal.

#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
use ashpd::desktop::settings::{ColorScheme, Settings};
#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
use futures_lite::stream::StreamExt;
#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
use std::sync::mpsc;
#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
use tokio::runtime::Handle;

/// Window appearance preference (light or dark mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAppearance {
    /// Light appearance (light theme).
    Light,
    /// Dark appearance (dark theme).
    Dark,
}

impl WindowAppearance {
    /// Convert from XDG Desktop Portal ColorScheme.
    fn from_native(cs: ColorScheme) -> Self {
        match cs {
            ColorScheme::PreferDark => WindowAppearance::Dark,
            ColorScheme::PreferLight => WindowAppearance::Light,
            ColorScheme::NoPreference => WindowAppearance::Light,
        }
    }
}

/// Events emitted by the XDG Desktop Portal monitor.
#[derive(Debug, Clone)]
pub enum XDPEvent {
    /// Window appearance (color scheme) changed.
    WindowAppearance(WindowAppearance),
    /// Cursor theme changed.
    CursorTheme(String),
    /// Cursor size changed.
    CursorSize(u32),
}

/// Event source for XDG Desktop Portal events.
///
/// This monitors system settings and emits events when they change.
/// Requires the `xdg-portal` feature and `tokio-runner` feature to be enabled.
#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
pub struct XDPEventSource {
    receiver: mpsc::Receiver<XDPEvent>,
}

#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
impl XDPEventSource {
    /// Create a new XDG Desktop Portal event source.
    ///
    /// This will spawn background tasks to monitor system settings.
    /// The task runner must be initialized (via `tasks::init()`).
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();

        // Spawn async task to monitor settings
        // Use tokio Handle directly to spawn detached background tasks
        if let Ok(handle) = Handle::try_current() {
            handle.spawn(async move {
            let settings = match Settings::new().await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Failed to initialize XDG Desktop Portal settings: {}", e);
                    return;
                }
            };

            // Read initial color scheme
            if let Ok(initial_appearance) = settings.color_scheme().await {
                let _ = sender.send(XDPEvent::WindowAppearance(
                    WindowAppearance::from_native(initial_appearance),
                ));
            }

            // Read initial cursor theme
            if let Ok(initial_theme) = settings
                .read::<String>("org.gnome.desktop.interface", "cursor-theme")
                .await
            {
                let _ = sender.send(XDPEvent::CursorTheme(initial_theme));
            }

            // Read initial cursor size
            // Note: Using i32 because u32 causes invalid type error in ashpd
            if let Ok(initial_size) = settings
                .read::<i32>("org.gnome.desktop.interface", "cursor-size")
                .await
            {
                let _ = sender.send(XDPEvent::CursorSize(initial_size as u32));
            }

            // Monitor cursor theme changes
            if let Ok(mut cursor_theme_changed) = settings
                .receive_setting_changed_with_args(
                    "org.gnome.desktop.interface",
                    "cursor-theme",
                )
                .await
            {
                let sender = sender.clone();
                if let Ok(handle) = Handle::try_current() {
                    handle.spawn(async move {
                        while let Some(theme) = cursor_theme_changed.next().await {
                            match theme {
                                Ok(theme) => {
                                    let _ = sender.send(XDPEvent::CursorTheme(theme));
                                }
                                Err(e) => {
                                    log::warn!("Error receiving cursor theme change: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
            }

            // Monitor cursor size changes
            if let Ok(mut cursor_size_changed) = settings
                .receive_setting_changed_with_args::<i32>(
                    "org.gnome.desktop.interface",
                    "cursor-size",
                )
                .await
            {
                let sender = sender.clone();
                if let Ok(handle) = Handle::try_current() {
                    handle.spawn(async move {
                        while let Some(size) = cursor_size_changed.next().await {
                            match size {
                                Ok(size) => {
                                    let _ = sender.send(XDPEvent::CursorSize(size as u32));
                                }
                                Err(e) => {
                                    log::warn!("Error receiving cursor size change: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
            }

            // Monitor color scheme changes
            let mut appearance_changed = match settings.receive_color_scheme_changed().await {
                Ok(stream) => stream,
                Err(e) => {
                    log::warn!("Failed to monitor color scheme changes: {}", e);
                    return;
                }
            };

            while let Some(scheme) = appearance_changed.next().await {
                let _ = sender.send(XDPEvent::WindowAppearance(
                    WindowAppearance::from_native(scheme),
                ));
            }
            });
        } else {
            log::warn!("XDG Desktop Portal: No tokio runtime available. Portal monitoring disabled.");
        }

        Self { receiver }
    }

    /// Try to receive the next event without blocking.
    ///
    /// Returns `None` if no event is available.
    pub fn try_recv(&self) -> Result<XDPEvent, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive the next event, blocking until one is available.
    ///
    /// This will block the current thread until an event is received.
    pub fn recv(&self) -> Result<XDPEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    /// Get a reference to the underlying receiver.
    pub fn receiver(&self) -> &mpsc::Receiver<XDPEvent> {
        &self.receiver
    }
}

#[cfg(all(target_os = "linux", feature = "xdg-portal"))]
impl Default for XDPEventSource {
    fn default() -> Self {
        Self::new()
    }
}

// Stub implementation when xdg-portal feature is disabled
#[cfg(not(all(target_os = "linux", feature = "xdg-portal")))]
pub struct XDPEventSource;

#[cfg(not(all(target_os = "linux", feature = "xdg-portal")))]
impl XDPEventSource {
    pub fn new() -> Self {
        Self
    }

    pub fn try_recv(&self) -> Result<XDPEvent, ()> {
        Err(())
    }

    pub fn recv(&self) -> Result<XDPEvent, ()> {
        Err(())
    }
}

#[cfg(not(all(target_os = "linux", feature = "xdg-portal")))]
impl Default for XDPEventSource {
    fn default() -> Self {
        Self::new()
    }
}

