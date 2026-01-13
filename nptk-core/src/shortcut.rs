// SPDX-License-Identifier: MIT OR Apache-2.0
//! Keyboard shortcut infrastructure for NPTK

use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use std::collections::HashMap;
use std::sync::Arc;
use crate::app::update::Update;

/// A keyboard shortcut consisting of a key code and modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// The physical key code
    pub key: KeyCode,
    /// Modifier keys that must be pressed
    pub modifiers: ModifiersState,
}

impl Shortcut {
    /// Create a new shortcut
    pub fn new(key: KeyCode, modifiers: ModifiersState) -> Self {
        Self { key, modifiers }
    }

    /// Create a shortcut with Ctrl modifier
    pub fn ctrl(key: KeyCode) -> Self {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::CONTROL, true);
        Self {
            key,
            modifiers: mods,
        }
    }

    /// Create a shortcut with Alt modifier
    pub fn alt(key: KeyCode) -> Self {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::ALT, true);
        Self {
            key,
            modifiers: mods,
        }
    }

    /// Create a shortcut with Shift modifier
    pub fn shift(key: KeyCode) -> Self {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::SHIFT, true);
        Self {
            key,
            modifiers: mods,
        }
    }

    /// Create a shortcut with Super/Command modifier
    pub fn super_key(key: KeyCode) -> Self {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::SUPER, true);
        Self {
            key,
            modifiers: mods,
        }
    }

    /// Create a shortcut with Ctrl+Shift modifiers
    pub fn ctrl_shift(key: KeyCode) -> Self {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::CONTROL, true);
        mods.set(ModifiersState::SHIFT, true);
        Self {
            key,
            modifiers: mods,
        }
    }

    /// Create a shortcut with Ctrl+Alt modifiers
    pub fn ctrl_alt(key: KeyCode) -> Self {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::CONTROL, true);
        mods.set(ModifiersState::ALT, true);
        Self {
            key,
            modifiers: mods,
        }
    }

    /// Check if a physical key and current modifier state matches this shortcut
    pub fn matches(&self, physical_key: &PhysicalKey, current_modifiers: ModifiersState) -> bool {
        if let PhysicalKey::Code(key_code) = physical_key {
            if *key_code != self.key {
                return false;
            }
            
            // Check that all required modifiers are pressed
            // (but allow extra modifiers to be pressed)
            self.modifiers.control_key() == current_modifiers.control_key()
                && self.modifiers.alt_key() == current_modifiers.alt_key()
                && self.modifiers.shift_key() == current_modifiers.shift_key()
                && self.modifiers.super_key() == current_modifiers.super_key()
        } else {
            false
        }
    }

    /// Format shortcut as a string (e.g., "Ctrl+N")
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        
        if self.modifiers.control_key() {
            parts.push("Ctrl");
        }
        if self.modifiers.alt_key() {
            parts.push("Alt");
        }
        if self.modifiers.shift_key() {
            parts.push("Shift");
        }
        if self.modifiers.super_key() {
            parts.push("Super");
        }
        
        let key_str = key_code_to_string(self.key);
        parts.push(key_str.as_str());
        
        parts.join("+")
    }
}

/// Convert a KeyCode to a display string
fn key_code_to_string(key: KeyCode) -> String {
    use KeyCode::*;
    String::from(match key {
        KeyA => "A",
        KeyB => "B",
        KeyC => "C",
        KeyD => "D",
        KeyE => "E",
        KeyF => "F",
        KeyG => "G",
        KeyH => "H",
        KeyI => "I",
        KeyJ => "J",
        KeyK => "K",
        KeyL => "L",
        KeyM => "M",
        KeyN => "N",
        KeyO => "O",
        KeyP => "P",
        KeyQ => "Q",
        KeyR => "R",
        KeyS => "S",
        KeyT => "T",
        KeyU => "U",
        KeyV => "V",
        KeyW => "W",
        KeyX => "X",
        KeyY => "Y",
        KeyZ => "Z",
        Digit1 => "1",
        Digit2 => "2",
        Digit3 => "3",
        Digit4 => "4",
        Digit5 => "5",
        Digit6 => "6",
        Digit7 => "7",
        Digit8 => "8",
        Digit9 => "9",
        Digit0 => "0",
        Escape => "Esc",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        Enter => "Enter",
        Tab => "Tab",
        Backspace => "Backspace",
        Delete => "Delete",
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        Insert => "Insert",
        _ => "?",
    })
}

/// Internal shortcut registry state
struct ShortcutRegistryState {
    /// Map from shortcuts to action callbacks
    shortcuts: HashMap<Shortcut, Arc<dyn Fn() -> Update + Send + Sync>>,
}

/// Registry for keyboard shortcuts and their associated actions
///
/// This is a thread-safe, cloneable wrapper around the internal registry state.
#[derive(Clone)]
pub struct ShortcutRegistry {
    state: Arc<std::sync::Mutex<ShortcutRegistryState>>,
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcutRegistry {
    /// Create a new shortcut registry
    pub fn new() -> Self {
        Self {
            state: Arc::new(std::sync::Mutex::new(ShortcutRegistryState {
                shortcuts: HashMap::new(),
            })),
        }
    }

    /// Register a shortcut with an action callback
    pub fn register<F>(&self, shortcut: Shortcut, action: F)
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        let mut state = self.state.lock().unwrap();
        state.shortcuts.insert(shortcut, Arc::new(action));
    }

    /// Unregister a shortcut
    pub fn unregister(&self, shortcut: &Shortcut) {
        let mut state = self.state.lock().unwrap();
        state.shortcuts.remove(shortcut);
    }

    /// Check if a shortcut is registered
    pub fn is_registered(&self, shortcut: &Shortcut) -> bool {
        let state = self.state.lock().unwrap();
        state.shortcuts.contains_key(shortcut)
    }

    /// Try to dispatch a shortcut based on a physical key and modifier state
    /// Returns the Update result if a matching shortcut was found and executed
    pub fn try_dispatch(&self, physical_key: &PhysicalKey, modifiers: ModifiersState) -> Option<Update> {
        let state = self.state.lock().unwrap();
        for (shortcut, action) in &state.shortcuts {
            if shortcut.matches(physical_key, modifiers) {
                return Some(action());
            }
        }
        None
    }

    /// Get all registered shortcuts (returns a clone of the shortcut list)
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        let state = self.state.lock().unwrap();
        state.shortcuts.keys().cloned().collect()
    }
}
