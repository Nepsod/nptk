// SPDX-License-Identifier: LGPL-3.0-only
//! AppMenu registrar client for window registration.

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::ObjectPath;
use zbus::Result as ZbusResult;

const REGISTRAR_BUS: &str = "com.canonical.AppMenu.Registrar";
const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";
const REGISTRAR_INTERFACE: &str = "com.canonical.AppMenu.Registrar";
const MENU_OBJECT_PATH: &str = "/com/canonical/menu/1";

/// Client for the AppMenu registrar service.
pub struct AppMenuRegistrar<'a> {
    proxy: Proxy<'a>,
    current: Option<u64>,
    service: String,
}

impl<'a> AppMenuRegistrar<'a> {
    pub fn new(connection: &'a Connection, service: String) -> ZbusResult<Self> {
        let proxy = Proxy::new(
            connection,
            REGISTRAR_BUS,
            REGISTRAR_PATH,
            REGISTRAR_INTERFACE,
        )?;
        Ok(Self {
            proxy,
            current: None,
            service,
        })
    }

    pub fn set_window(&mut self, window_id: Option<u64>) -> ZbusResult<bool> {
        if self.current == window_id {
            return Ok(false);
        }

        if let Some(id) = window_id {
            let path = ObjectPath::try_from(MENU_OBJECT_PATH)?;
            // Always use 3-arg version to ensure service name is registered.
            // This helps Plasma resolve the menu even if it only has the unique bus name.
            log::info!(
                "AppMenu registrar RegisterWindow 3-arg attempt window_id={} service={} path={}",
                id,
                self.service,
                MENU_OBJECT_PATH
            );
            let call3_with_string_path: ZbusResult<()> = self.proxy.call(
                "RegisterWindow",
                &((id as u32), self.service.as_str(), MENU_OBJECT_PATH),
            );
            let call3: ZbusResult<()> = if let Err(error) = call3_with_string_path {
                log::info!(
                    "AppMenu registrar RegisterWindow 3-arg (string path) failed for window_id={}: {}. Retrying with object path argument.",
                    id,
                    error
                );
                self.proxy.call(
                    "RegisterWindow",
                    &((id as u32), self.service.as_str(), path.clone()),
                )
            } else {
                Ok(())
            };
            if call3.is_err() {
                // Fall back to 2-arg if 3-arg fails (for compatibility)
                log::info!(
                    "AppMenu registrar RegisterWindow 3-arg failed for window_id={}, trying 2-arg",
                    id
                );
                log::debug!("3-arg RegisterWindow failed, trying 2-arg");
                let call2_with_string_path: ZbusResult<()> =
                    self.proxy.call("RegisterWindow", &((id as u32), MENU_OBJECT_PATH));
                let call2: ZbusResult<()> = if let Err(error) = call2_with_string_path {
                    log::info!(
                        "AppMenu registrar RegisterWindow 2-arg (string path) failed for window_id={}: {}. Retrying with object path argument.",
                        id,
                        error
                    );
                    self.proxy.call("RegisterWindow", &((id as u32), path))
                } else {
                    Ok(())
                };
                if let Err(error) = call2 {
                    return Err(error);
                }
                log::info!(
                    "AppMenu registrar RegisterWindow 2-arg success window_id={} path={}",
                    id,
                    MENU_OBJECT_PATH
                );
                log::debug!("Window registered with 2-arg RegisterWindow");
            } else {
                log::info!(
                    "AppMenu registrar RegisterWindow 3-arg success window_id={} service={} path={}",
                    id,
                    self.service,
                    MENU_OBJECT_PATH
                );
                log::debug!(
                    "Window registered with 3-arg RegisterWindow (service={})",
                    self.service
                );
            }
        } else if let Some(id) = self.current.take() {
            let _: () = self.proxy.call("UnregisterWindow", &((id as u32),))?;
        }

        self.current = window_id;
        Ok(true)
    }
}
