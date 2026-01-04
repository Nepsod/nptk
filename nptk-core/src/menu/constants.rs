//! Menu rendering constants
//!
//! Centralized constants for menu rendering used across the menu system.

/// Height of each menu item in pixels
pub const ITEM_HEIGHT: f64 = 24.0;

/// Top and bottom padding for the menu
pub const PADDING: f64 = 4.0;

/// Left and right padding for text in menu items
pub const TEXT_PADDING: f64 = 10.0;

/// Right padding for shortcuts
pub const SHORTCUT_RIGHT_PADDING: f64 = 12.0;

/// Minimum gap between text and shortcut to avoid overlap
pub const MIN_TEXT_SHORTCUT_GAP: f64 = 40.0;

/// Minimum width of the menu
pub const MIN_WIDTH: f64 = 120.0;

/// Maximum width of the menu
pub const MAX_WIDTH: f64 = 400.0;

/// Border radius for the menu background
pub const BORDER_RADIUS: f64 = 4.0;

/// Font size for menu item text
pub const FONT_SIZE: f64 = 14.0;

/// Label used to identify separator items
pub const SEPARATOR_LABEL: &str = "---";

/// Estimated pixels per character for regular text (used in width calculation)
pub const TEXT_CHAR_WIDTH: f64 = 7.0;

/// Estimated pixels per character for shortcut text (slightly wider for monospace shortcuts)
pub const SHORTCUT_CHAR_WIDTH: f64 = 8.0;

/// Size of the submenu arrow indicator
pub const ARROW_SIZE: f64 = 3.0;

/// Additional constants for widget-specific use (maintained for compatibility)
/// These may be removed in the future if widget code is refactored to use core constants

/// Top and bottom padding for the popup (widget-specific, matches PADDING * 2)
pub const POPUP_PADDING: f64 = 8.0;

/// Border radius for individual menu item backgrounds
pub const ITEM_BORDER_RADIUS: f64 = 2.0;

/// Top padding offset for items (from popup top)
pub const ITEM_TOP_PADDING: f64 = 4.0;

/// Horizontal padding for item text
pub const ITEM_TEXT_X_OFFSET: f64 = 8.0;

/// Vertical offset for item text
pub const ITEM_TEXT_Y_OFFSET: f64 = 2.0;

/// Horizontal offset for submenu arrow from right edge
pub const ARROW_X_OFFSET: f64 = 12.0;

/// Horizontal margin for item background
pub const ITEM_BG_MARGIN: f64 = 2.0;

/// Horizontal padding for separator lines
pub const SEPARATOR_PADDING: f64 = 8.0;

/// Small overlap between parent and child popup
pub const CHILD_POPUP_OVERLAP: f64 = 2.0;

/// Border stroke width
pub const BORDER_STROKE_WIDTH: f64 = 1.0;

/// Width reserved for checkmark or submenu indicator
pub const CHECKMARK_ARROW_WIDTH: f64 = 20.0;

/// Additional width reserved for text rendering (checkmark + spacing)
pub const TEXT_RENDERING_RESERVE: f64 = 30.0;

/// Vertical offset for item text (from item top)
pub const TEXT_Y_OFFSET: f64 = 4.0;
