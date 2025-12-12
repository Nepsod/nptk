// SPDX-License-Identifier: MIT OR Apache-2.0

//! Theme color extraction for menu popup widget

use nptk_core::vg::peniko::Color;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

/// Theme colors extracted from the theme for menu popup rendering
pub struct ThemeColors {
    /// Background color of the popup
    pub bg_color: Color,
    /// Border color of the popup
    pub border_color: Color,
    /// Text color for menu items
    pub text_color: Color,
    /// Color for disabled menu items
    pub disabled_color: Color,
    /// Background color for hovered menu items
    pub hovered_color: Color,
}

impl ThemeColors {
    /// Extract all theme colors for the menu popup widget
    pub fn extract(theme: &dyn Theme, widget_id: WidgetId) -> Self {
        let bg_color = theme
            .get_property(
                widget_id.clone(),
                &nptk_theme::properties::ThemeProperty::ColorBackground,
            )
            .unwrap_or_else(|| Color::from_rgb8(255, 255, 255));

        let border_color = theme
            .get_property(
                widget_id.clone(),
                &nptk_theme::properties::ThemeProperty::ColorBorder,
            )
            .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)); // Light gray border

        let text_color = theme
            .get_property(
                widget_id.clone(),
                &nptk_theme::properties::ThemeProperty::ColorText,
            )
            .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));

        let disabled_color = theme
            .get_property(
                widget_id.clone(),
                &nptk_theme::properties::ThemeProperty::ColorDisabled,
            )
            .unwrap_or_else(|| Color::from_rgb8(150, 150, 150));

        let hovered_color = theme
            .get_property(
                widget_id,
                &nptk_theme::properties::ThemeProperty::ColorMenuHovered,
            )
            .unwrap_or_else(|| Color::from_rgb8(180, 180, 180));

        Self {
            bg_color,
            border_color,
            text_color,
            disabled_color,
            hovered_color,
        }
    }
}
