#![cfg(target_os = "linux")]

//! Drag & drop support via wl_data_device_manager.

// Drag & drop requires wl_data_device_manager protocol
// This is a placeholder implementation

use super::shell::WaylandClientState;

// TODO: Implement drag & drop when wayland-protocols data device support is available
// This will require:
// - wl_data_device_manager
// - wl_data_device
// - wl_data_source
// - wl_data_offer
// - MIME type negotiation
// - Drag source and destination handling

// For now, this module is a placeholder

