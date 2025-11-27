//! DBus menu protocol implementation.
//!
//! This module provides the DBusMenu protocol implementation for global menu support.
//! It implements the `com.canonical.dbusmenu` interface and integrates with the
//! `com.canonical.AppMenu.Registrar` service.

mod bridge;
mod menu_object;
mod registrar;
mod types;

pub use bridge::{Bridge, BridgeEvent};
pub use types::{MenuSnapshot, RemoteMenuNode};


