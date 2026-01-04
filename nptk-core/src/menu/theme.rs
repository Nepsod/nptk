//! Theme color extraction for menu rendering
//!
//! Provides utilities for extracting theme colors for menu rendering,
//! ensuring consistent defaults and fallback behavior.

use nptk_theme::id::WidgetId;
use nptk_theme::properties::ThemeProperty;
use nptk_theme::theme::Theme;
use vello::peniko::Color;

/// Theme colors extracted from the theme for menu rendering
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
    /// Extract all theme colors for menu rendering
    ///
    /// Uses the provided `WidgetId` to look up theme-specific properties,
    /// falling back to default colors if properties are not found.
    pub fn extract(theme: &dyn Theme, widget_id: WidgetId) -> Self {
        let bg_color = theme
            .get_property(widget_id.clone(), &ThemeProperty::ColorBackground)
            .unwrap_or_else(|| Color::from_rgb8(255, 255, 255));

        let border_color = theme
            .get_property(widget_id.clone(), &ThemeProperty::ColorBorder)
            .unwrap_or_else(|| Color::from_rgb8(200, 200, 200));

        let text_color = theme
            .get_property(widget_id.clone(), &ThemeProperty::ColorText)
            .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));

        // Disabled color is derived from text color with reduced opacity
        let disabled_color = theme
            .get_property(widget_id.clone(), &ThemeProperty::ColorText)
            .map(|c| {
                // Make disabled color more transparent
                let components = c.components;
                let r = (components[0] * 255.0).clamp(0.0, 255.0) as u8;
                let g = (components[1] * 255.0).clamp(0.0, 255.0) as u8;
                let b = (components[2] * 255.0).clamp(0.0, 255.0) as u8;
                let alpha = (components[3] * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
                Color::from_rgba8(r, g, b, alpha)
            })
            .unwrap_or_else(|| Color::from_rgb8(128, 128, 128));

        let hovered_color = theme
            .get_property(widget_id, &ThemeProperty::ColorMenuHovered)
            .unwrap_or_else(|| Color::from_rgb8(230, 230, 230));

        Self {
            bg_color,
            border_color,
            text_color,
            disabled_color,
            hovered_color,
        }
    }
}
