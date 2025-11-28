#![cfg(target_os = "linux")]

//! Touch input handling.

use wayland_client::protocol::wl_touch;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use super::super::events::{InputEvent, TouchEvent};
use super::super::shell::WaylandClientState;

impl Dispatch<wl_touch::WlTouch, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        _touch: &wl_touch::WlTouch,
        event: wl_touch::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_touch::Event::Down {
                serial: _,
                time: _,
                surface,
                id,
                x,
                y,
            } => {
                let key = surface.id().protocol_id();
                if let Some(surface) = state.shared.get_surface(key) {
                    surface.push_input_event(InputEvent::Touch(TouchEvent::Down {
                        id,
                        surface_x: x,
                        surface_y: y,
                    }));
                    surface.request_redraw();
                }
            },
            wl_touch::Event::Up {
                serial: _,
                time: _,
                id,
            } => {
                // Find surface with this touch id - for simplicity, send to focused surface
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Touch(TouchEvent::Up { id }));
                        surface.request_redraw();
                    }
                }
            },
            wl_touch::Event::Motion {
                time: _,
                id,
                x,
                y,
            } => {
                // Find surface with this touch id - for simplicity, send to focused surface
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Touch(TouchEvent::Motion {
                            id,
                            surface_x: x,
                            surface_y: y,
                        }));
                        surface.request_redraw();
                    }
                }
            },
            wl_touch::Event::Frame => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Touch(TouchEvent::Frame));
                        surface.request_redraw();
                    }
                }
            },
            wl_touch::Event::Cancel => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Touch(TouchEvent::Cancel));
                        surface.request_redraw();
                    }
                }
            },
            _ => {},
        }
    }
}

