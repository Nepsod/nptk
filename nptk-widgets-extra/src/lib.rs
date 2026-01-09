// SPDX-License-Identifier: LGPL-3.0-only
//! Extra widgets for nptk.
//!
//! Contains additional widgets that require LGPL-3.0 licensing.

/// Contains the [expandable_section::ExpandableSection] widget.
pub mod expandable_section;

/// Contains the [file_icon::FileIcon] widget.
pub mod file_icon;

/// Contains the [menu_button::MenuButton] widget.
pub mod menu_button;

/// Contains the [menu_popup::MenuPopup] widget.
pub mod menu_popup;

/// Contains the [menubar::MenuBar] widget and global menu integration.
pub mod menubar;

/// Contains the [progress::Progress] widget.
pub mod progress;

/// Contains the [radio_button::RadioButton] widget.
pub mod radio_button;

/// Contains the [scroll_container::ScrollContainer] widget.
pub mod scroll_container;

/// Contains the [secret_input::SecretInput] widget.
pub mod secret_input;

/// Contains the [sidebar::Sidebar] widget.
pub mod sidebar;
pub use sidebar::{Sidebar, SidebarItem, SidebarSection};

/// Contains the [tabs_container::TabsContainer] widget.
pub mod tabs_container;

/// Contains the [text_input::TextInput] widget.
pub mod text_input;

/// Contains theme rendering bridge functionality.
pub mod theme_rendering;

/// Contains the [toggle::Toggle] widget.
pub mod toggle;

/// Contains the [toolbar::Toolbar] widget.
pub mod toolbar;

/// Contains the [value_input::ValueInput] widget.
pub mod value_input;
