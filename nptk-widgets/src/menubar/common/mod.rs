//! Common types and utilities for the menubar module.

/// Platform detection utilities.
pub mod platform {
    use nptk_core::vgi::Platform;

    /// Detect if we're running in a Wayland session.
    pub fn is_wayland_session() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|s| s.to_lowercase() == "wayland")
                .unwrap_or(false)
    }

    /// Detect the current platform.
    pub fn detect() -> Platform {
        Platform::detect()
    }

    /// Check if we're using native Wayland platform.
    pub fn is_native_wayland() -> bool {
        detect() == Platform::Wayland
    }
}

/// X11 window hints utilities.
#[cfg(feature = "global-menu")]
pub mod x11 {
    use std::ffi::CString;

    /// Set X11 window hints for Plasma appmenu discovery.
    ///
    /// This sets the `_KDE_NET_WM_APPMENU_SERVICE_NAME` and
    /// `_KDE_NET_WM_APPMENU_OBJECT_PATH` properties on the window.
    pub fn set_appmenu_hints(window_id: u32, service_name: &str, object_path: &str) -> Result<(), String> {
        use x11_dl::xlib::{Xlib, XA_STRING, PropModeReplace};

        let xlib = Xlib::open().map_err(|e| format!("Failed to load X11 library: {e}"))?;
        unsafe {
            let display = (xlib.XOpenDisplay)(std::ptr::null());
            if display.is_null() {
                return Err("Failed to open X11 display".to_string());
            }

            // Get atoms for the KDE appmenu properties
            let service_name_atom_cstr = CString::new("_KDE_NET_WM_APPMENU_SERVICE_NAME")
                .map_err(|e| format!("Failed to create CString: {e}"))?;
            let object_path_atom_cstr = CString::new("_KDE_NET_WM_APPMENU_OBJECT_PATH")
                .map_err(|e| format!("Failed to create CString: {e}"))?;

            let service_name_atom = (xlib.XInternAtom)(
                display,
                service_name_atom_cstr.as_ptr(),
                0, // only_if_exists = false
            );
            let object_path_atom = (xlib.XInternAtom)(
                display,
                object_path_atom_cstr.as_ptr(),
                0, // only_if_exists = false
            );

            if service_name_atom == 0 || object_path_atom == 0 {
                (xlib.XCloseDisplay)(display);
                return Err("Failed to intern X11 atoms".to_string());
            }

            // Set _KDE_NET_WM_APPMENU_SERVICE_NAME property
            let service_name_cstr = CString::new(service_name)
                .map_err(|e| format!("Failed to create service name CString: {e}"))?;
            (xlib.XChangeProperty)(
                display,
                window_id as u64,
                service_name_atom,
                XA_STRING,
                8, // format: 8 bits per element
                PropModeReplace,
                service_name_cstr.as_ptr() as *const u8,
                service_name_cstr.as_bytes().len() as i32,
            );

            // Set _KDE_NET_WM_APPMENU_OBJECT_PATH property
            let object_path_cstr = CString::new(object_path)
                .map_err(|e| format!("Failed to create object path CString: {e}"))?;
            (xlib.XChangeProperty)(
                display,
                window_id as u64,
                object_path_atom,
                XA_STRING,
                8, // format: 8 bits per element
                PropModeReplace,
                object_path_cstr.as_ptr() as *const u8,
                object_path_cstr.as_bytes().len() as i32,
            );

            // Flush to ensure properties are set
            (xlib.XFlush)(display);
            (xlib.XCloseDisplay)(display);

            log::info!(
                "Set X11 appmenu hints: service={}, path={} on window {}",
                service_name,
                object_path,
                window_id
            );
        }

        Ok(())
    }
}

#[cfg(not(feature = "global-menu"))]
pub mod x11 {
    pub fn set_appmenu_hints(_window_id: u32, _service_name: &str, _object_path: &str) -> Result<(), String> {
        Ok(())
    }
}

