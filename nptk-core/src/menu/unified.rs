//! Unified menu system core structures
//!
//! This module provides the core types for the unified menu system that works
//! for both menubar menus and context menus.

use crate::app::update::Update;
use crate::menu::commands::MenuCommand;
use std::sync::Arc;

/// Unified menu item that can be used in both menubar and context menus.
#[derive(Clone)]
pub struct MenuItem {
    /// Command identifier (enum for standard commands, u32 for custom)
    pub id: MenuCommand,
    /// Display label for the menu item
    pub label: String,
    /// Optional keyboard shortcut text (e.g., "Ctrl+N")
    pub shortcut: Option<String>,
    /// Whether the menu item is enabled/clickable
    pub enabled: bool,
    /// Whether the menu item is checked (for toggle items)
    pub checked: bool,
    /// Submenu items (if this is a submenu)
    pub submenu: Option<MenuTemplate>,
    /// Callback function to execute when the menu item is activated
    pub action: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
}

impl MenuItem {
    /// Create a new menu item with a command ID and label
    pub fn new(id: MenuCommand, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            shortcut: None,
            enabled: true,
            checked: false,
            submenu: None,
            action: None,
        }
    }

    /// Create a separator item
    pub fn separator() -> Self {
        Self {
            id: MenuCommand::Custom(0xFFFF),
            label: "---".to_string(),
            shortcut: None,
            enabled: false,
            checked: false,
            submenu: None,
            action: None,
        }
    }

    /// Check if this is a separator
    pub fn is_separator(&self) -> bool {
        self.label.trim() == "---" || matches!(self.id, MenuCommand::Custom(0xFFFF))
    }

    /// Set the keyboard shortcut
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set checked state
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set submenu
    pub fn with_submenu(mut self, submenu: MenuTemplate) -> Self {
        self.submenu = Some(submenu);
        self
    }

    /// Set action callback
    pub fn with_action<F>(mut self, action: F) -> Self
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        self.action = Some(Arc::new(action));
        self
    }

    /// Check if this item has a submenu
    pub fn has_submenu(&self) -> bool {
        self.submenu.is_some()
    }
}

/// Reusable menu template that can be instantiated in different contexts.
#[derive(Clone)]
pub struct MenuTemplate {
    /// Template identifier (for reference/merging)
    pub id: String,
    /// Menu items in this template
    pub items: Vec<MenuItem>,
}

impl MenuTemplate {
    /// Create a new menu template
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
        }
    }

    /// Add a menu item to this template
    pub fn add_item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items
    pub fn add_items(mut self, items: Vec<MenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Find an item by command ID
    pub fn find_item_mut(&mut self, cmd: MenuCommand) -> Option<&mut MenuItem> {
        self.items.iter_mut().find(|item| item.id == cmd)
    }

    /// Find an item by command ID (immutable)
    pub fn find_item(&self, cmd: MenuCommand) -> Option<&MenuItem> {
        self.items.iter().find(|item| item.id == cmd)
    }

    /// Create from a vector of items
    pub fn from_items(id: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            id: id.into(),
            items,
        }
    }
}

/// Context information for enabling/disabling menu items dynamically.
#[derive(Clone, Debug, Default)]
pub struct MenuContext {
    /// Number of selected items
    pub selection_count: usize,
    /// Whether items can be copied
    pub can_copy: bool,
    /// Whether items can be moved/cut
    pub can_move: bool,
    /// Whether paste operation is available
    pub can_paste: bool,
    /// Whether undo is available
    pub can_undo: bool,
    /// Current view mode (for View menu items)
    pub view_mode: Option<ViewMode>,
    /// Whether auto-arrange is enabled
    pub auto_arrange: bool,
    /// Custom context data (extension point)
    pub custom: std::collections::HashMap<String, bool>,
}

/// View mode enumeration (for View menu commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Icon,
    SmallIcon,
    List,
    Details,
}

impl MenuContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context for background (no selection)
    pub fn background() -> Self {
        Self {
            selection_count: 0,
            can_copy: false,
            can_move: false,
            can_paste: true, // Paste might be available from clipboard
            can_undo: true,  // Undo might be available
            view_mode: None,
            auto_arrange: false,
            custom: Default::default(),
        }
    }

    /// Create a context for selection
    pub fn selection(count: usize, can_copy: bool, can_move: bool) -> Self {
        Self {
            selection_count: count,
            can_copy,
            can_move,
            can_paste: false,
            can_undo: true,
            view_mode: None,
            auto_arrange: false,
            custom: Default::default(),
        }
    }

    /// Set view mode
    pub fn with_view_mode(mut self, mode: ViewMode) -> Self {
        self.view_mode = Some(mode);
        self
    }

    /// Set auto-arrange state
    pub fn with_auto_arrange(mut self, enabled: bool) -> Self {
        self.auto_arrange = enabled;
        self
    }

    /// Set custom context flag
    pub fn set_custom(&mut self, key: impl Into<String>, value: bool) {
        self.custom.insert(key.into(), value);
    }

    /// Get custom context flag
    pub fn get_custom(&self, key: &str) -> bool {
        self.custom.get(key).copied().unwrap_or(false)
    }
}
