//! Theme color extraction for menu rendering
//!
//! Provides utilities for extracting theme colors for menu rendering,
//! ensuring consistent defaults and fallback behavior.

use crate::theme::{ColorRole, Palette};
use vello::peniko::Color;

/// Theme colors extracted from the palette for menu rendering
pub struct MenuThemeColors {
    /// Background color of the menu
    pub bg_color: Color,
    /// Border color of the menu
    pub border_color: Color,
    /// Text color for menu items
    pub text_color: Color,
    /// Color for disabled menu items
    pub disabled_color: Color,
    /// Background color for hovered menu items
    pub hovered_color: Color,
}

impl MenuThemeColors {
    /// Extract all theme colors for menu rendering from palette
    ///
    /// Uses palette color roles (based on SerenityOS):
    /// - MenuBase for background
    /// - MenuBaseText for text
    /// - MenuSelection for hovered background
    /// - MenuSelectionText for hovered text
    /// - DisabledTextFront for disabled text
    pub fn extract_from_palette(palette: &Palette) -> Self {
        let bg_color = palette.color(ColorRole::MenuBase);
        let text_color = palette.color(ColorRole::MenuBaseText);
        let hovered_color = palette.color(ColorRole::MenuSelection);
        
        // Border color - use a darker shade of menu base or threed shadow
        let border_color = palette.color(ColorRole::ThreedShadow1);
        
        // Disabled color - use DisabledTextFront or derive from text color with reduced opacity
        let disabled_color = palette.color(ColorRole::DisabledTextFront);

        Self {
            bg_color,
            border_color,
            text_color,
            disabled_color,
            hovered_color,
        }
    }
    
}
