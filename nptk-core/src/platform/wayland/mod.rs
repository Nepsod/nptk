#![cfg(target_os = "linux")]

//! Modular Wayland platform implementation.
//!
//! This module provides a refactored, modular implementation of Wayland client
//! functionality, organized by responsibility.

pub mod activation;
pub mod client;
pub mod clipboard;
pub mod core;
pub mod data_device;
pub mod decoration;
pub mod events;
pub mod fractional_scale;
pub mod globals;
pub mod idle;
pub mod input;
pub mod presentation;
pub mod primary_selection;
pub mod shell;
pub mod surface;
pub mod text_input;
pub mod viewporter;
pub mod window;

// Re-export commonly used types
pub use client::{WaylandClient, WaylandQueueHandle};
pub use globals::WaylandGlobals;
pub use surface::{WaylandSurface, WaylandSurfaceInner};

// Events are pub(crate) for internal use only
