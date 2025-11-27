#![cfg(target_os = "linux")]

//! Decoration protocol handling.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_protocols_plasma::server_decoration::client::{
    org_kde_kwin_server_decoration, org_kde_kwin_server_decoration_manager,
};

use super::client::SharedState;
use super::shell::WaylandClientState;

impl Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _proxy: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _event: zxdg_decoration_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Decoration manager doesn't send events to clients
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, u32> for WaylandClientState {
    fn event(
        state: &mut Self,
        _proxy: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        surface_key: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let Some(surface) = state.shared.get_surface(*surface_key) {
            match event {
                zxdg_toplevel_decoration_v1::Event::Configure { .. } => {
                    // Decoration mode/config changed; request a redraw so chrome can update.
                    surface.request_redraw();
                },
                _ => {},
            }
        }
    }
}

impl Dispatch<org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_server_decoration_manager::OrgKdeKwinServerDecorationManager,
        _event: org_kde_kwin_server_decoration_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // KDE decoration manager doesn't send events to clients
    }
}

impl Dispatch<org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_server_decoration::OrgKdeKwinServerDecoration,
        _event: org_kde_kwin_server_decoration::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // KDE decoration doesn't send events to clients
    }
}

#[cfg(feature = "global-menu")]
use wayland_protocols_plasma::appmenu::client::{
    org_kde_kwin_appmenu, org_kde_kwin_appmenu_manager,
};

#[cfg(feature = "global-menu")]
impl Dispatch<org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager,
        _event: org_kde_kwin_appmenu_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The appmenu manager doesn't send events to clients
    }
}

#[cfg(feature = "global-menu")]
impl Dispatch<org_kde_kwin_appmenu::OrgKdeKwinAppmenu, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _proxy: &org_kde_kwin_appmenu::OrgKdeKwinAppmenu,
        _event: org_kde_kwin_appmenu::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The appmenu object doesn't send events to clients
    }
}

