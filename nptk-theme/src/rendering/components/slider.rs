use vello::peniko::Color;

use crate::id::WidgetId;
use crate::theme::Theme;

use super::super::state::WidgetState;

/// Visual description of slider track and thumb colors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderVisual {
    /// Track fill color.
    pub track: Color,
    /// Thumb/handle color.
    pub thumb: Color,
}

/// Derive a [SliderVisual] from theme properties.
pub fn slider_visual<T: Theme + ?Sized>(
    theme: &T,
    id: &WidgetId,
    state: WidgetState,
) -> SliderVisual {
    let track = theme
        .get_property(id.clone(), &crate::properties::ThemeProperty::SliderTrack)
        .unwrap_or_else(|| default_track_color(state));

    let thumb = theme
        .get_property(id.clone(), &crate::properties::ThemeProperty::SliderThumb)
        .unwrap_or_else(|| default_thumb_color(state));

    SliderVisual { track, thumb }
}

fn default_track_color(state: WidgetState) -> Color {
    if state.is_disabled() {
        Color::from_rgb8(150, 150, 150)
    } else {
        Color::from_rgb8(220, 220, 220)
    }
}

fn default_thumb_color(state: WidgetState) -> Color {
    if state.is_disabled() {
        Color::from_rgb8(150, 150, 150)
    } else if state.is_pressed() {
        Color::from_rgb8(100, 150, 255)
    } else {
        Color::from_rgb8(180, 180, 180)
    }
}
