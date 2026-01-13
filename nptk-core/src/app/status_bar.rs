// SPDX-License-Identifier: MIT OR Apache-2.0
//! Status bar text management system for NPTK
//!
//! Provides a simple system for widgets to update status bar text (e.g., when hovering over toolbar buttons).

use crate::signal::state::StateSignal;
use crate::signal::Signal;

/// Manages the current status bar text.
///
/// This is a thread-safe, cloneable wrapper around status bar text state.
/// Status bar widgets can read the current text, and widgets (like buttons)
/// can update it when hovered.
#[derive(Clone)]
pub struct StatusBarManager {
    text: StateSignal<String>,
}

impl StatusBarManager {
    /// Create a new status bar manager with empty initial text.
    pub fn new(initial_text: String) -> Self {
        Self {
            text: StateSignal::new(initial_text),
        }
    }

    /// Set the status bar text.
    pub fn set_text(&self, text: String) {
        self.text.set(text);
    }

    /// Clear the status bar text (set to empty string).
    pub fn clear(&self) {
        self.text.set(String::new());
    }

    /// Get the current status bar text signal.
    ///
    /// Status bar widgets can use this to reactively display the current text.
    pub fn text_signal(&self) -> StateSignal<String> {
        self.text.clone()
    }

    /// Get the current status bar text.
    pub fn get_text(&self) -> String {
        self.text.get().clone()
    }
}

impl Default for StatusBarManager {
    fn default() -> Self {
        Self::new(String::new())
    }
}
