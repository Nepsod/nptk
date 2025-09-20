use nalgebra::Vector2;
use winit::event::{DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, Ime};
use winit::keyboard::ModifiersState;

use crate::app::diagnostics::Diagnostics;
use crate::app::focus::SharedFocusManager;
use crate::app::font_ctx::FontContext;

/// The application information container.
pub struct AppInfo {
    /// The position of the cursor. If [None], the cursor left the window.
    pub cursor_pos: Option<Vector2<f64>>,
    /// The fired key events.
    pub keys: Vec<(DeviceId, KeyEvent)>,
    /// The fired mouse button events.
    pub buttons: Vec<(DeviceId, MouseButton, ElementState)>,
    /// The mouse scroll delta, if a [winit::event::WindowEvent::MouseWheel] event was fired.
    pub mouse_scroll_delta: Option<MouseScrollDelta>,
    /// Current modifier keys state.
    pub modifiers: ModifiersState,
    /// IME (Input Method Editor) events for text composition.
    pub ime_events: Vec<Ime>,
    /// App Diagnostics.
    pub diagnostics: Diagnostics,
    /// The current font context.
    pub font_context: FontContext,
    /// The size of the window.
    pub size: Vector2<f64>,
    /// Focus manager for tracking widget focus state.
    pub focus_manager: SharedFocusManager,
}

impl AppInfo {
    /// Reset the application information for a new frame.
    pub fn reset(&mut self) {
        self.buttons.clear();
        self.keys.clear();
        self.mouse_scroll_delta = None;
        self.ime_events.clear();
    }
}

impl Default for AppInfo {
    fn default() -> Self {
        use crate::app::focus::{FocusManager};
        use std::sync::{Arc, Mutex};
        
        Self {
            cursor_pos: None,
            keys: Vec::with_capacity(4),
            buttons: Vec::with_capacity(2),
            mouse_scroll_delta: None,
            modifiers: ModifiersState::default(),
            ime_events: Vec::new(),
            diagnostics: Diagnostics::default(),
            font_context: FontContext::default(),
            size: Vector2::new(0.0, 0.0),
            focus_manager: Arc::new(Mutex::new(FocusManager::new())),
        }
    }
}
