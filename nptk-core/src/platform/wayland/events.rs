#![cfg(target_os = "linux")]

//! Unified event types for all Wayland input.

use wayland_client::protocol::{wl_keyboard, wl_pointer};

/// Unified input event enum for all input types.
#[derive(Debug, Clone)]
pub(crate) enum InputEvent {
    Pointer(PointerEvent),
    Keyboard(KeyboardEvent),
    Ime(ImeEvent),
    Touch(TouchEvent),
    Tablet(TabletEvent),
}

/// Pointer input events.
#[derive(Debug, Clone)]
pub(crate) enum PointerEvent {
    Enter {
        surface_x: f64,
        surface_y: f64,
    },
    Leave,
    Motion {
        surface_x: f64,
        surface_y: f64,
    },
    Button {
        button: u32,
        state: wl_pointer::ButtonState,
    },
    Axis {
        horizontal: Option<f64>,
        vertical: Option<f64>,
    },
    AxisSource {
        source: wl_pointer::AxisSource,
    },
    AxisStop,
    AxisDiscrete {
        axis: wl_pointer::Axis,
        discrete: i32,
    },
    AxisValue120 {
        axis: wl_pointer::Axis,
        value120: i32,
    },
    Frame,
}

/// Keyboard input events.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum KeyboardEvent {
    Enter,
    Leave,
    Key {
        keycode: u32,
        state: wl_keyboard::KeyState,
    },
    Modifiers {
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    },
    RepeatInfo {
        rate: i32,
        delay: i32,
    },
    Keymap {
        keymap_string: String,
    },
}

/// IME input events.
#[derive(Debug, Clone)]
pub(crate) enum ImeEvent {
    Preedit {
        text: String,
        cursor_begin: Option<u32>,
        cursor_end: Option<u32>,
    },
    Commit {
        text: String,
    },
    Done,
}

/// Touch input events.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum TouchEvent {
    Down {
        id: i32,
        surface_x: f64,
        surface_y: f64,
    },
    Up {
        id: i32,
    },
    Motion {
        id: i32,
        surface_x: f64,
        surface_y: f64,
    },
    Frame,
    Cancel,
}

/// Tablet input events.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum TabletEvent {
    Tool { tool_id: u32 },
    Proximity { tool_id: u32, x: f64, y: f64 },
    Motion { x: f64, y: f64 },
    Pressure { pressure: u32 },
    Button { button: u32, state: u32 },
    Frame,
}
