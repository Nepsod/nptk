//! Public API for Wayland appmenu integration.
//!
//! This module provides a public interface for setting application menus
//! on Wayland surfaces. The actual implementation is in the `wayland` module.
//!
//! This module also provides `MenuInfoStorage` for storing menu service information
//! that can be accessed by platform integrations to discover and set application menus.

#![cfg(all(target_os = "linux", feature = "global-menu"))]

use std::sync::{Mutex, OnceLock};
use wayland_client::protocol::wl_surface;

/// Menu information storage for platform integration.
///
/// This provides a global storage mechanism for application menu
/// service name and object path, which can be used by platform-specific
/// integrations (e.g., Plasma's compositor) to discover menus.
pub struct MenuInfoStorage;

struct Storage {
    service_name: Option<String>,
    object_path: Option<String>,
}

static STORAGE: OnceLock<Mutex<Storage>> = OnceLock::new();

impl MenuInfoStorage {
    fn get_storage() -> &'static Mutex<Storage> {
        STORAGE.get_or_init(|| Mutex::new(Storage {
            service_name: None,
            object_path: None,
        }))
    }

    /// Set the menu service name and object path.
    ///
    /// # Arguments
    /// * `service_name` - The D-Bus service name (e.g., "com.nptk.menubar.app_12345")
    /// * `object_path` - The D-Bus object path (e.g., "/com/canonical/menu/1")
    pub fn set(service_name: String, object_path: String) {
        let storage = Self::get_storage();
        let mut guard = storage.lock().unwrap();
        guard.service_name = Some(service_name);
        guard.object_path = Some(object_path);
        log::debug!(
            "Menu info stored: service={}, path={}",
            guard.service_name.as_ref().unwrap(),
            guard.object_path.as_ref().unwrap()
        );
    }

    /// Get the menu service name and object path.
    ///
    /// # Returns
    /// * `Some((service_name, object_path))` if menu info is available
    /// * `None` if menu info has not been set yet
    pub fn get() -> Option<(String, String)> {
        let storage = Self::get_storage();
        let guard = storage.lock().unwrap();
        match (&guard.service_name, &guard.object_path) {
            (Some(service), Some(path)) => Some((service.clone(), path.clone())),
            _ => None,
        }
    }
}

/// Set the application menu for a Wayland surface.
///
/// This function should be called by the menubar module when menu info is available.
/// It uses the platform's Wayland client to set the appmenu protocol.
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
    use crate::platform::wayland::client::WaylandClient;
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
    use crate::platform::wayland::client::WaylandClient;
    let client = WaylandClient::instance();
    client.update_appmenu_for_all_surfaces(service, path);
}


