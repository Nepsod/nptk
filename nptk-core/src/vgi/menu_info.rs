//! Global menu information storage for Wayland window management.
//!
//! This module provides a global storage mechanism for application menu
//! service name and object path, which can be used by Plasma's compositor
//! to discover menus on Wayland. This works with both winit-based and
//! native Wayland implementations.

use std::sync::{Mutex, OnceLock};

/// Global storage for menu information.
struct MenuInfo {
    service_name: Option<String>,
    object_path: Option<String>,
}

static MENU_INFO: OnceLock<Mutex<MenuInfo>> = OnceLock::new();

fn get_menu_info_storage() -> &'static Mutex<MenuInfo> {
    MENU_INFO.get_or_init(|| Mutex::new(MenuInfo {
        service_name: None,
        object_path: None,
    }))
}

/// Set the menu service name and object path.
///
/// This should be called by the global menu bridge when registering
/// the menu with the registrar. The information is then available
/// for Plasma's compositor to discover the menu on Wayland.
///
/// # Arguments
/// * `service_name` - The D-Bus service name (e.g., "com.nptk.menubar.app_12345")
/// * `object_path` - The D-Bus object path (e.g., "/com/canonical/menu/1")
pub fn set_menu_info(service_name: String, object_path: String) {
    let info = get_menu_info_storage();
    let mut guard = info.lock().unwrap();
    guard.service_name = Some(service_name);
    guard.object_path = Some(object_path);
    log::debug!("Menu info stored: service={}, path={}", guard.service_name.as_ref().unwrap(), guard.object_path.as_ref().unwrap());
}

/// Get the menu service name and object path.
///
/// # Returns
/// * `Some((service_name, object_path))` if menu info is available
/// * `None` if menu info has not been set yet
pub fn get_menu_info() -> Option<(String, String)> {
    let info = get_menu_info_storage();
    let guard = info.lock().unwrap();
    match (&guard.service_name, &guard.object_path) {
        (Some(service), Some(path)) => Some((service.clone(), path.clone())),
        _ => None,
    }
}

/// Notify the Wayland client that menu info has been updated.
///
/// This should be called after `set_menu_info()` to ensure the Wayland client
/// is aware of the menu information and can set appmenu for existing surfaces.
///
/// This function is safe to call even if the Wayland client is not initialized
/// or the wayland feature is not enabled. It only attempts to connect on actual Wayland sessions.
#[cfg(all(target_os = "linux", feature = "global-menu"))]
pub fn notify_wayland_client() {
    // Only try to notify Wayland client if we're actually on a Wayland session
    // Check WAYLAND_DISPLAY to avoid trying to connect on X11
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        log::debug!("Not on Wayland session (WAYLAND_DISPLAY not set), skipping Wayland client notification");
        return;
    }

    // Try to get the Wayland client instance and update it with menu info
    // We use catch_unwind because the client might not be initialized yet
    let result = std::panic::catch_unwind(|| {
        #[cfg(feature = "wayland")]
        {
            use crate::vgi::wl_client::WaylandClient;
            if let Some((service, path)) = get_menu_info() {
                let client = WaylandClient::instance();
                client.set_menu_info(service, path);
            }
        }
    });
    
    if let Err(e) = result {
        log::debug!("Failed to notify Wayland client of menu info update: {:?}. This is expected if wayland feature is not enabled or client is not initialized.", e);
    }
}

#[cfg(not(all(target_os = "linux", feature = "global-menu")))]
pub fn notify_wayland_client() {
    // No-op when not on Linux or global-menu feature is disabled
}

