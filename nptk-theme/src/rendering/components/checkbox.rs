use peniko::Color;

use crate::id::WidgetId;
use crate::theme::Theme;

use super::super::primitives::{checkbox_mapping, themed_color_or};
use super::super::state::{CheckboxState, WidgetState};

/// Visual representation for checkbox rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckboxVisual {
    /// Fill color of the checkbox body.
    pub background: Color,
    /// Border color used to outline the checkbox.
    pub border: Color,
    /// Color applied to the checkmark/indeterminate glyph.
    pub symbol: Color,
}

/// Compute a [CheckboxVisual] from the given theme and widget state.
pub fn checkbox_visual<T: Theme + ?Sized>(
    theme: &T,
    id: &WidgetId,
    state: WidgetState,
    checkbox_state: CheckboxState,
) -> CheckboxVisual {
    let mapping = checkbox_mapping(checkbox_state);
    let background = themed_color_or(theme, id, mapping.property, mapping.fallback);
    let border = default_border_color(state);
    let symbol = default_symbol_color(checkbox_state);
    CheckboxVisual {
        background,
        border,
        symbol,
    }
}

fn default_border_color(state: WidgetState) -> Color {
    if state.is_disabled() {
        Color::from_rgb8(150, 150, 150)
    } else {
        Color::from_rgb8(200, 200, 200)
    }
}

fn default_symbol_color(state: CheckboxState) -> Color {
    match state {
        CheckboxState::Unchecked => Color::TRANSPARENT,
        CheckboxState::Checked | CheckboxState::Indeterminate => Color::WHITE,
    }
}
