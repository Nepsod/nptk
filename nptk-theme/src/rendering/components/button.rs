use peniko::Color;

use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::Theme;

use super::super::primitives::{button_mapping, themed_color_or};
use super::super::state::WidgetState;

/// Aggregated styling information required to paint a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonVisual {
    /// Fill color for the control background.
    pub background: Color,
    /// Text color rendered atop the button.
    pub text: Color,
    /// Border color defining the button outline.
    pub border: Color,
    /// Optional focus ring color if focus styling is active.
    pub focus_ring: Option<Color>,
}

impl ButtonVisual {
    /// Construct a new [ButtonVisual] with explicit styling values.
    pub fn new(background: Color, text: Color, border: Color, focus_ring: Option<Color>) -> Self {
        Self {
            background,
            text,
            border,
            focus_ring,
        }
    }
}

/// Build a [ButtonVisual] by consulting the provided theme.
pub fn button_visual<T: Theme + ?Sized>(
    theme: &T,
    id: &WidgetId,
    state: WidgetState,
) -> ButtonVisual {
    let mapping = button_mapping(state);
    let background = themed_color_or(theme, id, mapping.property, mapping.fallback);
    let text = theme
        .get_property(id.clone(), &ThemeProperty::Text)
        .unwrap_or_else(|| default_text_color(state));
    let border = theme
        .get_property(id.clone(), &ThemeProperty::ColorBorder)
        .unwrap_or_else(|| default_border_color(state));
    let focus_ring = if state.is_focused() {
        Some(Color::from_rgb8(100, 150, 255))
    } else {
        None
    };

    ButtonVisual::new(background, text, border, focus_ring)
}

fn default_text_color(state: WidgetState) -> Color {
    if state.is_disabled() {
        Color::from_rgb8(170, 170, 170)
    } else {
        Color::from_rgb8(30, 30, 30)
    }
}

fn default_border_color(state: WidgetState) -> Color {
    if state.is_pressed() {
        Color::from_rgb8(140, 140, 140)
    } else {
        Color::from_rgb8(190, 190, 190)
    }
}
