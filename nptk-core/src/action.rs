// SPDX-License-Identifier: LGPL-3.0-only
//! # Action System
//!
//! This module provides a system for managing user actions and their associated properties.

use std::sync::Arc;
use crate::app::update::Update;
use crate::signal::state::StateSignal;
use crate::signal::Signal;

/// Represents a reusable user action that can be bound to menus, toolbars, and shortcuts.
///
/// Actions are reactive; updating their properties (text, enabled, checked) will automatically
/// update all UI elements subscribed to them.
#[derive(Clone)]
pub struct Action {
    /// Unique identifier for this action (optional, helpful for registries)
    pub id: String,
    
    /// Display text (e.g. "Save")
    pub text: StateSignal<String>,
    
    /// Icon name (e.g. "document-save")
    pub icon: StateSignal<Option<String>>,
    
    /// Whether the action is enabled
    pub enabled: StateSignal<bool>,
    
    /// Whether the action is checkable (toggle)
    pub checkable: bool,
    
    /// Whether the action is currently checked
    pub checked: StateSignal<bool>,
    
    /// Keyboard shortcut string (e.g. "Ctrl+S")
    pub shortcut: StateSignal<Option<String>>,
    
    /// Status tip / Tooltip
    pub status_tip: StateSignal<Option<String>>,
    
    /// Callback to execute when triggered
    pub on_triggered: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
}

impl Action {
    /// Create a new action with default values
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: StateSignal::new(text.into()),
            icon: StateSignal::new(None),
            enabled: StateSignal::new(true),
            checkable: false,
            checked: StateSignal::new(false),
            shortcut: StateSignal::new(None),
            status_tip: StateSignal::new(None),
            on_triggered: None,
        }
    }
    
    /// Set the icon
    pub fn with_icon(self, icon: impl Into<String>) -> Self {
        self.icon.set(Some(icon.into()));
        self
    }
    
    /// Set the shortcut
    pub fn with_shortcut(self, shortcut: impl Into<String>) -> Self {
        self.shortcut.set(Some(shortcut.into()));
        self
    }
    
    /// Set status tip
    pub fn with_status_tip(self, tip: impl Into<String>) -> Self {
        self.status_tip.set(Some(tip.into()));
        self
    }
    
    /// Set checkable
    pub fn checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }
    
    /// Set checked state
    pub fn with_checked(self, checked: bool) -> Self {
        self.checked.set(checked);
        self
    }
    
    /// Set enabled state
    pub fn with_enabled(self, enabled: bool) -> Self {
        self.enabled.set(enabled);
        self
    }
    
    /// Set trigger handler
    pub fn on_triggered<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        self.on_triggered = Some(Arc::new(f));
        self
    }
    
    /// Trigger the action (execute callback and toggle state if checkable)
    pub fn trigger(&self) -> Update {
        if !*self.enabled.get() {
            return Update::empty();
        }
        
        // Auto-toggle if checkable
        if self.checkable {
            let current = *self.checked.get();
            self.checked.set(!current);
        }
        
        if let Some(cb) = &self.on_triggered {
            cb()
        } else {
            Update::empty()
        }
    }
}

use std::fmt;
impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Action")
            .field("id", &self.id)
            .field("text", &self.text.get())
            .field("enabled", &self.enabled.get())
            .field("checked", &self.checked.get())
            .finish()
    }
}
