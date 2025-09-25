#![warn(missing_docs)]

//! Core library for nptk => See `nptk` crate.
//!
//! Contains core app logic and widget types.

#[cfg(feature = "vg")]
pub use vello as vg;

#[cfg(feature = "vg")]
pub use skrifa;

/// Contains useful types for interacting with winit.
pub mod window {
    pub use winit::event::*;
    pub use winit::event_loop::*;
    pub use winit::keyboard::*;
    pub use winit::window::*;
}

/// Contains app functionality.
pub mod app;

/// Contains the [MayConfig](config::MayConfig) struct.
pub mod config;

/// Contains useful types and functions for layout interaction.
pub mod layout;

/// Contains the signal system for reactive programming
pub mod signal;

/// Contains the core widget functionalities
pub mod widget;

/// Contains structures to work with the component architecture
pub mod component;

/// Contains the task runner and utilities for running async
pub mod tasks;

/// Contains the [reference::Ref] for representing a reference to a value.
pub mod reference;

/// Contains the plugin system.
pub mod plugin;

/// Contains focus management functionality.
pub mod focus;

/// Contains text input processing and text buffer management.
pub mod text_input;

/// Contains text rendering functionality using Parley.
pub mod text_render;

/// Re-export BrushIndex for use in other modules
pub use text_render::BrushIndex;

#[cfg(feature = "include-noto-sans")]
pub const DEFAULT_FONT: &[u8] = include_bytes!("NotoSans.ttf");
