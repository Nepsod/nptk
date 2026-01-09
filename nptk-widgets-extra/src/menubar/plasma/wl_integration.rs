// SPDX-License-Identifier: LGPL-3.0-only
//! Integration with VGI's Wayland client for appmenu support.
//!
//! This module provides functions to integrate the menubar's menu info
//! with the VGI Wayland client, allowing appmenu to be set on Wayland surfaces.

#![cfg(all(target_os = "linux", feature = "global-menu"))]

use wayland_client::protocol::wl_surface;

use nptk_core::platform::MenuInfoStorage;

/// Set the application menu for a Wayland surface using platform's public appmenu API.
///
/// This function uses platform's public appmenu API to set the appmenu on a surface.
/// It reads menu info from the menubar module's MenuInfoStorage.
///
/// # Arguments
/// * `surface` - The Wayland surface to set the appmenu on
///
/// # Returns
/// * `Ok(())` if the appmenu was set successfully
/// * `Err(String)` if there was an error (e.g., appmenu manager not available, menu info not set)
pub fn set_appmenu_for_surface_via_wl_client(
    surface: &wl_surface::WlSurface,
) -> Result<(), String> {
    // Get menu info from menubar module
    let Some((service, path)) = MenuInfoStorage::get() else {
        return Err("Menu info not available yet".to_string());
    };

    // Use platform's public appmenu API
    nptk_core::platform::appmenu::set_appmenu_for_surface(surface, service, path)
}

/// Notify VGI's Wayland client that menu info has been updated.
///
/// This should be called whenever menu info changes in the menubar module.
/// It will update appmenu for all existing surfaces.
pub fn notify_wl_client_menu_update() {
    let Some((service, path)) = MenuInfoStorage::get() else {
        log::debug!("Menu info not available, skipping Wayland client notification");
        return;
    };

    // Use platform's public appmenu API
    nptk_core::platform::appmenu::update_appmenu_for_all_surfaces(service, path);
}
