// SPDX-License-Identifier: LGPL-3.0-only
//! Shared helpers for Linux global menu integration.

use std::sync::{Mutex, OnceLock};

pub(crate) struct MenuInfoStorage;

struct MenuInfo {
    service_name: Option<String>,
    object_path: Option<String>,
}

static MENU_INFO: OnceLock<Mutex<MenuInfo>> = OnceLock::new();

impl MenuInfoStorage {
    fn storage() -> &'static Mutex<MenuInfo> {
        MENU_INFO.get_or_init(|| {
            Mutex::new(MenuInfo {
                service_name: None,
                object_path: None,
            })
        })
    }

    pub(crate) fn set(service_name: String, object_path: String) {
        let mut guard = Self::storage()
            .lock()
            .expect("global menu info storage lock");
        guard.service_name = Some(service_name);
        guard.object_path = Some(object_path);
    }

    pub(crate) fn get() -> Option<(String, String)> {
        let guard = Self::storage()
            .lock()
            .expect("global menu info storage lock");
        match (&guard.service_name, &guard.object_path) {
            (Some(service_name), Some(object_path)) => {
                Some((service_name.clone(), object_path.clone()))
            }
            _ => None,
        }
    }
}

pub(crate) mod platform {
    pub(crate) fn is_wayland_session() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|value| value.eq_ignore_ascii_case("wayland"))
                .unwrap_or(false)
    }
}

#[cfg(feature = "x11")]
pub(crate) mod x11 {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, PropMode, Window};
    use x11rb::rust_connection::RustConnection;
    use x11rb::wrapper::ConnectionExt as _;

    pub(crate) fn set_appmenu_hints(
        window_id: u32,
        service_name: &str,
        object_path: &str,
    ) -> Result<(), String> {
        let (connection, _) =
            RustConnection::connect(None).map_err(|error| format!("X11 connect failed: {error}"))?;

        let service_atom = connection
            .intern_atom(false, b"_KDE_NET_WM_APPMENU_SERVICE_NAME")
            .map_err(|error| format!("X11 intern atom request failed: {error}"))?
            .reply()
            .map_err(|error| format!("X11 intern atom reply failed: {error}"))?
            .atom;

        let path_atom = connection
            .intern_atom(false, b"_KDE_NET_WM_APPMENU_OBJECT_PATH")
            .map_err(|error| format!("X11 intern atom request failed: {error}"))?
            .reply()
            .map_err(|error| format!("X11 intern atom reply failed: {error}"))?
            .atom;

        let window = window_id as Window;
        connection
            .change_property8(
                PropMode::REPLACE,
                window,
                service_atom,
                AtomEnum::STRING,
                service_name.as_bytes(),
            )
            .map_err(|error| format!("X11 set service property failed: {error}"))?;
        connection
            .change_property8(
                PropMode::REPLACE,
                window,
                path_atom,
                AtomEnum::STRING,
                object_path.as_bytes(),
            )
            .map_err(|error| format!("X11 set object path property failed: {error}"))?;
        connection
            .flush()
            .map_err(|error| format!("X11 flush failed: {error}"))?;

        Ok(())
    }
}

#[cfg(not(feature = "x11"))]
pub(crate) mod x11 {
    pub(crate) fn set_appmenu_hints(
        _window_id: u32,
        _service_name: &str,
        _object_path: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}
