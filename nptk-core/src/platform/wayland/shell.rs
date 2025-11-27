#![cfg(target_os = "linux")]

//! XDG shell protocol handling.

use wayland_client::protocol::wl_callback;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use super::client::SharedState;
use super::surface::WaylandSurfaceInner;
use std::sync::Arc;

// Import input modules to register their dispatch implementations
use super::input::keyboard;
use super::input::pointer;
use super::input::touch;
use super::input::seat;

// Import core protocol dispatches
use super::core;

/// Wayland client state for dispatch implementations.
pub struct WaylandClientState {
    pub(crate) shared: Arc<SharedState>,
}

impl WaylandClientState {
    pub fn new(shared: Arc<SharedState>) -> Self {
        Self { shared }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        xdg_surf: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                // Look up surface by xdg_surface's associated wl_surface
                // We need to find which surface has this xdg_surface
                let xdg_surf_id = xdg_surf.id().protocol_id();
                let surfaces_map = state.shared.surfaces().lock().unwrap();
                let surface_key = surfaces_map.iter()
                    .find_map(|(key, surface_weak)| {
                        surface_weak.upgrade().and_then(|s| {
                            if s.xdg_surface.id().protocol_id() == xdg_surf_id {
                                Some(*key)
                            } else {
                                None
                            }
                        })
                    });
                drop(surfaces_map);
                
                if let Some(surface_key) = surface_key {
                    log::trace!(
                        "Wayland xdg_surface configure serial={} (surface_key={})",
                        serial,
                        surface_key
                    );
                    xdg_surf.ack_configure(serial);
                    match conn.flush() {
                        Ok(()) => {
                            log::trace!("Wayland connection.flush() succeeded after ack");
                        },
                        Err(err) => {
                            log::warn!("Wayland flush error after ACK: {:?}", err);
                        },
                    }
                    if let Some(surface) = state.shared.get_surface(surface_key) {
                        log::trace!(
                            "Wayland post-ack invoking present for surface_key={}",
                            surface_key
                        );
                        surface.handle_configure_after_ack(serial);
                    } else {
                        log::warn!(
                            "Wayland surface not found for key={} in xdg_surface::Configure",
                            surface_key
                        );
                    }
                } else {
                    log::warn!("Wayland xdg_surface configure: could not find associated surface");
                }
            },
            _ => {},
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Look up surface by xdg_toplevel
        let toplevel_id = toplevel.id().protocol_id();
        let surfaces_map = state.shared.surfaces().lock().unwrap();
        let surface_key = surfaces_map.iter()
            .find_map(|(key, surface_weak)| {
                surface_weak.upgrade().and_then(|s| {
                    if s.xdg_toplevel.id().protocol_id() == toplevel_id {
                        Some(*key)
                    } else {
                        None
                    }
                })
            });
        drop(surfaces_map);
        
        if let Some(surface_key) = surface_key {
            if let Some(surface) = state.shared.get_surface(surface_key) {
                match event {
                    xdg_toplevel::Event::Configure { width, height, .. } => {
                        log::debug!(
                            "Wayland XdgToplevel({}) configure {}x{}",
                            surface_key,
                            width,
                            height
                        );
                        surface.handle_toplevel_configure(width, height);
                    },
                    xdg_toplevel::Event::Close => {
                        log::debug!("Wayland XdgToplevel({}) close", surface_key);
                        surface.mark_closed();
                    },
                    _ => {},
                }
            }
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            // Look up surface by callback ID
            let callback_id = callback.id().protocol_id();
            let callback_map = state.shared.callback_to_surface().lock().unwrap();
            if let Some(&surface_key) = callback_map.get(&callback_id) {
                drop(callback_map);
                log::trace!("Wayland: Frame done for surface {}", surface_key);
                if let Some(surface) = state.shared.get_surface(surface_key) {
                    surface.handle_frame_done();
                    // Remove callback mapping after use
                    let mut callback_map = state.shared.callback_to_surface().lock().unwrap();
                    callback_map.remove(&callback_id);
                }
            } else {
                log::warn!("Wayland: Frame callback {} not found in mapping", callback_id);
            }
        }
    }
}

