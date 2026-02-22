// SPDX-License-Identifier: LGPL-3.0-only
//! Extra widgets for nptk.
//!
//! Contains additional widgets that require LGPL-3.0 licensing.

pub mod choice;

/// Contains shared helper functions for input widgets.
mod input_helpers;

/// Contains the [expandable_section::ExpandableSection] widget.
pub mod expandable_section;

/// Contains the [breadcrumbs::Breadcrumbs] widget.
pub mod breadcrumbs;

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

/// Contains the [spacer::Spacer] widget.
pub mod spacer;
pub use spacer::Spacer;

/// Contains flex layout helpers: [flex::HStack], [flex::VStack], [flex::Expanded], [flex::Flexible].
pub mod flex;
pub use flex::{HStack, VStack, Expanded, Flexible};

/// Contains the [grid::Grid] widget and [grid::GridItem] types.
pub mod grid;
pub use grid::{Grid, GridItem};

/// Contains the [layout_builder::LayoutBuilder] widget.
pub mod layout_builder;
pub use layout_builder::LayoutBuilder;

/// Contains the [adaptive::Adaptive] widget for responsive layouts.
pub mod adaptive;
pub use adaptive::Adaptive;

/// Contains the [wrap::Wrap] widget for wrapping layouts.
pub mod wrap;
pub use wrap::Wrap;

/// Contains the [tabs_container::TabsContainer] widget.
pub mod tabs_container;

/// Contains the [text_input::TextInput] widget.
pub mod text_input;

/// Contains theme rendering bridge functionality.
mod theme_rendering;

/// Contains the [toggle::Toggle] widget.
pub mod toggle;

/// Contains the [toolbar::Toolbar] widget.
pub mod toolbar;

/// Contains the [value_input::ValueInput] widget.
pub mod value_input;
