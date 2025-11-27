#![cfg(target_os = "linux")]

//! Wayland input handling modules.

pub mod keyboard;
pub mod pointer;
pub mod touch;
pub mod tablet;
pub mod seat;

