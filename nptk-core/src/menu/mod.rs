//! Unified menu system
//!
//! This module provides a unified menu system that works for both menubar menus
//! and context menus, allowing menu items to be shared between different contexts.

pub mod commands;
pub mod constants;
pub mod context;
pub mod manager;
pub mod render;
pub mod templates;
pub mod theme;
pub mod unified;

// Re-export core types
pub use commands::MenuCommand;
pub use constants::*;
pub use context::ContextMenuState;
pub use manager::MenuManager;
pub use render::{render_menu, calculate_menu_size, MenuGeometry};
pub use templates::{init_edit_commands, init_view_menu, merge_menus};
pub use theme::MenuThemeColors;
pub use unified::{MenuItem, MenuTemplate, MenuContext, ViewMode};
