//! Public API for Wayland appmenu integration.
//!
//! This module provides a public interface for setting application menus
//! on Wayland surfaces. The actual implementation is in the private `wl_client` module.

#![cfg(all(target_os = "linux", feature = "global-menu"))]

use wayland_client::protocol::wl_surface;

/// Set the application menu for a Wayland surface.
///
/// This function should be called by the menubar module when menu info is available.
/// It uses VGI's internal Wayland client to set the appmenu protocol.
///
/// # Arguments
/// * `surface` - The Wayland surface to set the appmenu on
/// * `service` - The D-Bus service name for the menu
/// * `path` - The D-Bus object path for the menu
///
/// # Returns
/// * `Ok(())` if the appmenu was set successfully
/// * `Err(String)` if there was an error (e.g., appmenu manager not available)
pub fn set_appmenu_for_surface(
    surface: &wl_surface::WlSurface,
    service: String,
    path: String,
) -> Result<(), String> {
    use crate::vgi::wl_client::WaylandClient;
    let client = WaylandClient::instance();
    client.set_appmenu_for_surface_with_info(surface, service, path)
}

/// Update appmenu for all existing surfaces when menu info changes.
///
/// This should be called by the menubar module whenever menu info is updated.
///
/// # Arguments
/// * `service` - The D-Bus service name for the menu
/// * `path` - The D-Bus object path for the menu
pub fn update_appmenu_for_all_surfaces(service: String, path: String) {
    use crate::vgi::wl_client::WaylandClient;
    let client = WaylandClient::instance();
    client.update_appmenu_for_all_surfaces(service, path);
}

