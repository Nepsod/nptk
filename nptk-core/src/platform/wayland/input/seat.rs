#![cfg(target_os = "linux")]

//! Seat management.

use wayland_client::protocol::wl_seat;
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::super::shell::WaylandClientState;

impl Dispatch<wl_seat::WlSeat, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _seat: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No-op - seat events are handled by individual input devices
    }
}
