//! Context menu state management

use crate::menu::unified::MenuTemplate;
use std::sync::{Arc, Mutex};
use vello::kurbo::Point;

/// Manages the state of active context menus
#[derive(Clone, Default)]
pub struct ContextMenuState {
    state: Arc<Mutex<ContextMenuStack>>,
}

#[derive(Default)]
struct ContextMenuStack {
    stack: Vec<(MenuTemplate, Point)>,
}

impl ContextMenuState {
    /// Create a new context menu state manager
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ContextMenuStack::default())),
        }
    }

    /// Show a context menu at the given position
    pub fn show(&self, template: MenuTemplate, position: Point) {
        let mut state = self.state.lock().unwrap();
        state.stack.clear();
        state.stack.push((template, position));
    }

    /// Push a submenu to the stack
    pub fn push(&self, template: MenuTemplate, position: Point) {
        let mut state = self.state.lock().unwrap();
        state.stack.push((template, position));
    }

    /// Close all context menus
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.stack.clear();
    }

    /// Set the menu stack
    pub fn set_stack(&self, stack: Vec<(MenuTemplate, Point)>) {
        let mut state = self.state.lock().unwrap();
        state.stack = stack;
    }

    /// Get the active menu and position
    pub fn get_active(&self) -> Option<(MenuTemplate, Point)> {
        let state = self.state.lock().unwrap();
        state.stack.last().cloned()
    }

    /// Get the entire menu stack
    pub fn get_stack(&self) -> Vec<(MenuTemplate, Point)> {
        let state = self.state.lock().unwrap();
        state.stack.clone()
    }

    /// Check if any context menu is open
    pub fn is_open(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.stack.is_empty()
    }
}
