#![cfg(target_os = "linux")]

//! Text input support via zwp_text_input_manager_v3.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};

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
        _state: &mut Self,
        _text_input: &zwp_text_input_v3::ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_text_input_v3::Event::Enter { surface: _ } => {
                // Text input entered surface
                // TODO: Handle enter
            }
            zwp_text_input_v3::Event::Leave { surface: _ } => {
                // Text input left surface
                // TODO: Handle leave
            }
            zwp_text_input_v3::Event::PreeditString { text: _, cursor_begin: _, cursor_end: _ } => {
                // IME preedit update
                // TODO: Update preedit state
            }
            zwp_text_input_v3::Event::CommitString { text: _ } => {
                // IME commit
                // TODO: Commit text
            }
            zwp_text_input_v3::Event::DeleteSurroundingText { before_length: _, after_length: _ } => {
                // Delete surrounding text
                // TODO: Handle deletion
            }
            zwp_text_input_v3::Event::Done { serial: _ } => {
                // Transaction done
                // TODO: Apply changes
            }
            _ => {}
        }
    }
}
