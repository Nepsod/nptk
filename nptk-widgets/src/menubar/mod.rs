//! Menu bar widget with global menu integration.
//!
//! This module provides the `MenuBar` widget and related functionality for
//! creating application menu bars with support for global menu integration
//! on Linux desktop environments.

#[cfg(feature = "global-menu")]
mod common;
#[cfg(feature = "global-menu")]
mod dbus;
#[cfg(feature = "global-menu")]
mod plasma;

mod widget;
#[cfg(feature = "global-menu")]
mod adapter;

// Re-export the main widget
pub use widget::MenuBar;

// Re-export types for global menu integration
#[cfg(feature = "global-menu")]
pub use dbus::{Bridge, BridgeEvent, MenuSnapshot, RemoteMenuNode};
