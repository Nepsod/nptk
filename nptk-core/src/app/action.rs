// SPDX-License-Identifier: MIT OR Apache-2.0
//! Action callback system for status tips and hover tracking

use crate::app::update::Update;
use crate::menu::commands::MenuCommand;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Callback function type for action enter/leave events
pub type ActionCallback = Arc<dyn Fn() + Send + Sync>;

/// Manages action callbacks for enter/leave events
#[derive(Clone, Default)]
pub struct ActionCallbackManager {
    /// Map from MenuCommand to enter callbacks
    enter_callbacks: Arc<Mutex<HashMap<u32, Vec<ActionCallback>>>>,
    /// Map from MenuCommand to leave callbacks
    leave_callbacks: Arc<Mutex<HashMap<u32, Vec<ActionCallback>>>>,
}

impl ActionCallbackManager {
    /// Create a new action callback manager
    pub fn new() -> Self {
        Self {
            enter_callbacks: Arc::new(Mutex::new(HashMap::new())),
            leave_callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a callback for when an action is entered (hovered)
    pub fn register_enter<F>(&self, command: MenuCommand, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = command.to_u32();
        let mut callbacks = self.enter_callbacks.lock().unwrap();
        callbacks.entry(id).or_insert_with(Vec::new).push(Arc::new(callback));
    }

    /// Register a callback for when an action is left (no longer hovered)
    pub fn register_leave<F>(&self, command: MenuCommand, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = command.to_u32();
        let mut callbacks = self.leave_callbacks.lock().unwrap();
        callbacks.entry(id).or_insert_with(Vec::new).push(Arc::new(callback));
    }

    /// Trigger enter callbacks for a command
    pub fn trigger_enter(&self, command: MenuCommand) {
        let id = command.to_u32();
        let callbacks = self.enter_callbacks.lock().unwrap();
        if let Some(callbacks) = callbacks.get(&id) {
            for callback in callbacks {
                callback();
            }
        }
    }

    /// Trigger leave callbacks for a command
    pub fn trigger_leave(&self, command: MenuCommand) {
        let id = command.to_u32();
        let callbacks = self.leave_callbacks.lock().unwrap();
        if let Some(callbacks) = callbacks.get(&id) {
            for callback in callbacks {
                callback();
            }
        }
    }

    /// Unregister all callbacks for a command
    pub fn unregister(&self, command: MenuCommand) {
        let id = command.to_u32();
        let mut enter = self.enter_callbacks.lock().unwrap();
        let mut leave = self.leave_callbacks.lock().unwrap();
        enter.remove(&id);
        leave.remove(&id);
    }
}
