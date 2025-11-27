#![cfg(target_os = "linux")]

//! Modular Wayland VGI implementation.
//!
//! This module provides a refactored, modular implementation of Wayland client
//! functionality, organized by responsibility.

pub mod client;
pub mod core;
pub mod events;
pub mod globals;
pub mod surface;
pub mod shell;
pub mod window;
pub mod decoration;
pub mod input;
pub mod data_device;
pub mod clipboard;
pub mod viewporter;
pub mod fractional_scale;
pub mod presentation;
pub mod idle;

// Re-export commonly used types
pub use client::{WaylandClient, WaylandQueueHandle};
pub use globals::WaylandGlobals;
pub use surface::WaylandSurfaceInner;

// Events are pub(crate) for internal use only
pub(crate) use events::{InputEvent, KeyboardEvent, PointerEvent, TouchEvent, TabletEvent};

