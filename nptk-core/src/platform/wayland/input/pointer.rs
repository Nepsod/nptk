#![cfg(target_os = "linux")]

//! Pointer input handling.

use wayland_client::protocol::wl_pointer;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use super::super::events::{InputEvent, PointerEvent};
use super::super::shell::WaylandClientState;

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                serial: _,
                surface_x,
                surface_y,
                ..
            } => {
                let key = surface.id().protocol_id();
                state.shared.set_focused_surface(Some(key));
                if let Some(surface) = state.shared.get_surface(key) {
                    surface.push_input_event(InputEvent::Pointer(PointerEvent::Enter {
                        surface_x,
                        surface_y,
                    }));
                    surface.request_redraw();
                }
            },
            wl_pointer::Event::Leave { .. } => {
                let focused = state.shared.get_focused_surface_key();
                if let Some(key) = focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Pointer(PointerEvent::Leave));
                        surface.request_redraw();
                    }
                }
                state.shared.set_focused_surface(None);
            },
            wl_pointer::Event::Motion {
                time: _,
                surface_x,
                surface_y,
            } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Pointer(PointerEvent::Motion {
                            surface_x,
                            surface_y,
                        }));
                        surface.request_redraw();
                    }
                }
            },
            wl_pointer::Event::Button {
                serial: _,
                time: _,
                button,
                state: button_state,
            } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(button_state) = button_state.into_result() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::Button {
                                button,
                                state: button_state,
                            }));
                            surface.request_redraw();
                        }
                    }
                }
            },
            wl_pointer::Event::Axis {
                time: _,
                axis,
                value,
            } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(axis_kind) = axis.into_result() {
                            let event = match axis_kind {
                                wl_pointer::Axis::VerticalScroll => PointerEvent::Axis {
                                    horizontal: None,
                                    vertical: Some(value),
                                },
                                wl_pointer::Axis::HorizontalScroll => PointerEvent::Axis {
                                    horizontal: Some(value),
                                    vertical: None,
                                },
                                _ => PointerEvent::Axis {
                                    horizontal: None,
                                    vertical: None,
                                },
                            };
                            surface.push_input_event(InputEvent::Pointer(event));
                            surface.request_redraw();
                        }
                    }
                }
            },
            wl_pointer::Event::AxisSource { axis_source } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(source) = axis_source.into_result() {
                            surface.push_input_event(InputEvent::Pointer(
                                PointerEvent::AxisSource { source },
                            ));
                            surface.request_redraw();
                        }
                    }
                }
            },
            wl_pointer::Event::AxisStop { time: _, axis } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if axis.into_result().is_ok() {
                            surface.push_input_event(InputEvent::Pointer(PointerEvent::AxisStop));
                            surface.request_redraw();
                        }
                    }
                }
            },
            wl_pointer::Event::AxisDiscrete { axis, discrete } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(axis_kind) = axis.into_result() {
                            surface.push_input_event(InputEvent::Pointer(
                                PointerEvent::AxisDiscrete {
                                    axis: axis_kind,
                                    discrete,
                                },
                            ));
                            surface.request_redraw();
                        }
                    }
                }
            },
            wl_pointer::Event::AxisValue120 { axis, value120 } => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        if let Ok(axis_kind) = axis.into_result() {
                            surface.push_input_event(InputEvent::Pointer(
                                PointerEvent::AxisValue120 {
                                    axis: axis_kind,
                                    value120,
                                },
                            ));
                            surface.request_redraw();
                        }
                    }
                }
            },
            wl_pointer::Event::Frame => {
                if let Some(key) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Pointer(PointerEvent::Frame));
                        surface.request_redraw();
                    }
                }
            },
            _ => {},
        }
    }
}
