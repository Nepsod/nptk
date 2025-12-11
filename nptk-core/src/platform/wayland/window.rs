#![cfg(target_os = "linux")]

//! Window state management (minimize, maximize, fullscreen).

use wayland_protocols::xdg::shell::client::xdg_toplevel;

/// Window state flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Maximized,
    Fullscreen,
    Resizing,
    Activated,
}

/// Window state management for XDG toplevel surfaces.
///
/// Provides methods to request window state changes like maximize, fullscreen, and minimize.
pub struct WindowStateManager {
    // Window state will be tracked per surface
}

impl WindowStateManager {
    /// Request to maximize the window.
    pub fn set_maximized(toplevel: &xdg_toplevel::XdgToplevel) {
        toplevel.set_maximized();
    }

    /// Request to unmaximize the window.
    pub fn unset_maximized(toplevel: &xdg_toplevel::XdgToplevel) {
        toplevel.unset_maximized();
    }

    /// Request to make the window fullscreen.
    pub fn set_fullscreen(toplevel: &xdg_toplevel::XdgToplevel) {
        toplevel.set_fullscreen(None);
    }

    /// Request to exit fullscreen.
    pub fn unset_fullscreen(toplevel: &xdg_toplevel::XdgToplevel) {
        toplevel.unset_fullscreen();
    }

    /// Request to minimize the window.
    pub fn set_minimized(toplevel: &xdg_toplevel::XdgToplevel) {
        toplevel.set_minimized();
    }
}

// Window state is tracked via xdg_toplevel::Event::Configure states
// The actual state management will be handled in the shell module
// when processing configure events
