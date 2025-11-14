use peniko::Color;

use crate::id::WidgetId;
use crate::theme::Theme;

use super::super::state::WidgetState;

/// Visual data used when drawing text input controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextInputVisual {
    /// Field background color.
    pub background: Color,
    /// Border color for the input outline.
    pub border: Color,
    /// Caret/selection handle color.
    pub caret: Color,
}

/// Assemble a [TextInputVisual] with theme-aware colors.
pub fn text_input_visual<T: Theme + ?Sized>(
    theme: &T,
    id: &WidgetId,
    state: WidgetState,
) -> TextInputVisual {
    let background = theme
        .get_property(
            id.clone(),
            &crate::properties::ThemeProperty::ColorBackground,
        )
        .unwrap_or_else(|| default_background(state));

    let border = theme
        .get_property(id.clone(), &crate::properties::ThemeProperty::ColorBorder)
        .unwrap_or_else(|| default_border(state));

    let caret = theme
        .get_property(id.clone(), &crate::properties::ThemeProperty::ColorCursor)
        .unwrap_or_else(|| default_caret(state));

    TextInputVisual {
        background,
        border,
        caret,
    }
}

fn default_background(state: WidgetState) -> Color {
    if state.is_disabled() {
        Color::from_rgb8(150, 150, 150)
    } else {
        Color::from_rgb8(255, 255, 255)
    }
}

fn default_border(state: WidgetState) -> Color {
    if state.is_focused() {
        Color::from_rgb8(100, 150, 255)
    } else {
        Color::from_rgb8(200, 200, 200)
    }
}

fn default_caret(state: WidgetState) -> Color {
    if state.is_disabled() {
        Color::from_rgb8(170, 170, 170)
    } else {
        Color::from_rgb8(30, 30, 30)
    }
}
