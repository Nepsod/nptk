#![cfg(target_os = "linux")]

//! Text input support via zwp_text_input_manager_v3.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};

use super::events::{ImeEvent, InputEvent};
use super::shell::WaylandClientState;

/// Wrapper around the Wayland text input object.
pub struct TextInput {
    /// The underlying Wayland text input object.
    pub object: zwp_text_input_v3::ZwpTextInputV3,
}

impl TextInput {
    /// Create a new TextInput wrapper.
    pub fn new(object: zwp_text_input_v3::ZwpTextInputV3) -> Self {
        Self { object }
    }
}

impl Dispatch<zwp_text_input_manager_v3::ZwpTextInputManagerV3, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        _event: zwp_text_input_manager_v3::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for manager
    }
}

impl Dispatch<zwp_text_input_v3::ZwpTextInputV3, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        _text_input: &zwp_text_input_v3::ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_text_input_v3::Event::Enter { surface: _ } => {
                // Text input entered surface
            },
            zwp_text_input_v3::Event::Leave { surface: _ } => {
                // Text input left surface
            },
            zwp_text_input_v3::Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                let focused = state.shared.get_focused_surface_key();
                if let Some(key) = focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Ime(ImeEvent::Preedit {
                            text: text.unwrap_or_default(),
                            cursor_begin: if cursor_begin == -1 { None } else { Some(cursor_begin as u32) },
                            cursor_end: if cursor_end == -1 { None } else { Some(cursor_end as u32) },
                        }));
                        surface.request_redraw();
                    }
                }
            },
            zwp_text_input_v3::Event::CommitString { text } => {
                let focused = state.shared.get_focused_surface_key();
                if let Some(key) = focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Ime(ImeEvent::Commit {
                            text: text.unwrap_or_default(),
                        }));
                        surface.request_redraw();
                    }
                }
            },
            zwp_text_input_v3::Event::DeleteSurroundingText {
                before_length: _,
                after_length: _,
            } => {
                // TODO: Handle deletion if needed by application
            },
            zwp_text_input_v3::Event::Done { serial: _ } => {
                let focused = state.shared.get_focused_surface_key();
                if let Some(key) = focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Ime(ImeEvent::Done));
                        surface.request_redraw();
                    }
                }
            },
            _ => {},
        }
    }
}
