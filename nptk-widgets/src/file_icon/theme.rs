//! Theme-related helpers for file icon widget.

use nptk_core::vg::peniko::Color;
use nptk_theme::id::WidgetId;
use nptk_theme::properties::ThemeProperty;
use nptk_theme::theme::Theme;

use crate::file_icon::constants::{DEFAULT_TEXT_COLOR_B, DEFAULT_TEXT_COLOR_G, DEFAULT_TEXT_COLOR_R};

/// Extract icon color from theme, falling back to default if unavailable.
pub fn get_icon_color(theme: &mut dyn Theme, widget_id: WidgetId) -> Color {
    theme
        .get_property(widget_id, &ThemeProperty::ColorText)
        .or_else(|| theme.get_default_property(&ThemeProperty::ColorText))
        .unwrap_or(Color::from_rgb8(
            DEFAULT_TEXT_COLOR_R,
            DEFAULT_TEXT_COLOR_G,
            DEFAULT_TEXT_COLOR_B,
        ))
}
