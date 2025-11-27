#![cfg(target_os = "linux")]

//! Viewporter support via wp_viewporter.

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

use super::shell::WaylandClientState;

pub struct Viewport {
    pub object: wp_viewport::WpViewport,
}

impl Viewport {
    pub fn new(object: wp_viewport::WpViewport) -> Self {
        Self { object }
    }
}

impl Dispatch<wp_viewporter::WpViewporter, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _manager: &wp_viewporter::WpViewporter,
        _event: wp_viewporter::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for manager
    }
}

impl Dispatch<wp_viewport::WpViewport, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _viewport: &wp_viewport::WpViewport,
        _event: wp_viewport::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for viewport
    }
}
