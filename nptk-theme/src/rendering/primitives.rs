//! Shared color mapping primitives for theme rendering.
use vello::peniko::Color;

use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::Theme;

use super::state::{CheckboxState, WidgetState};

/// Associates a theme property with a fallback color.
#[derive(Clone, Copy)]
pub struct PropertyMapping {
    /// The property looked up in the current theme.
    pub property: ThemeProperty,
    /// Fallback color if the property is missing.
    pub fallback: Color,
}

/// Fetch a color property with a fallback in case of missing values.
pub fn themed_color_or<T: Theme + ?Sized>(
    theme: &T,
    id: &WidgetId,
    property: ThemeProperty,
    fallback: Color,
) -> Color {
    theme
        .get_property(id.clone(), &property)
        .unwrap_or(fallback)
}

/// Map a [WidgetState] to its corresponding button property lookup.
pub fn button_mapping(state: WidgetState) -> PropertyMapping {
    match state {
        WidgetState::Normal => PropertyMapping {
            property: ThemeProperty::ColorIdle,
            fallback: Color::from_rgb8(200, 200, 200),
        },
        WidgetState::Hovered | WidgetState::SelectedHovered => PropertyMapping {
            property: ThemeProperty::ColorHovered,
            fallback: Color::from_rgb8(180, 180, 180),
        },
        WidgetState::Pressed | WidgetState::SelectedPressed => PropertyMapping {
            property: ThemeProperty::ColorPressed,
            fallback: Color::from_rgb8(160, 160, 160),
        },
        WidgetState::Released | WidgetState::FocusedReleased | WidgetState::SelectedReleased => {
            PropertyMapping {
                property: ThemeProperty::ColorHovered,
                fallback: Color::from_rgb8(180, 180, 180),
            }
        },
        WidgetState::Focused | WidgetState::FocusedHovered | WidgetState::Selected => {
            PropertyMapping {
                property: ThemeProperty::ColorFocused,
                fallback: Color::from_rgb8(100, 150, 255),
            }
        },
        WidgetState::FocusedPressed => PropertyMapping {
            property: ThemeProperty::ColorPressed,
            fallback: Color::from_rgb8(80, 130, 235),
        },
        WidgetState::Disabled => PropertyMapping {
            property: ThemeProperty::ColorDisabled,
            fallback: Color::from_rgb8(150, 150, 150),
        },
    }
}

/// Map a [CheckboxState] to the appropriate color lookup.
pub fn checkbox_mapping(state: CheckboxState) -> PropertyMapping {
    match state {
        CheckboxState::Unchecked => PropertyMapping {
            property: ThemeProperty::ColorUnchecked,
            fallback: Color::from_rgb8(255, 255, 255),
        },
        CheckboxState::Checked => PropertyMapping {
            property: ThemeProperty::ColorChecked,
            fallback: Color::from_rgb8(100, 150, 255),
        },
        CheckboxState::Indeterminate => PropertyMapping {
            property: ThemeProperty::ColorIndeterminate,
            fallback: Color::from_rgb8(150, 150, 150),
        },
    }
}
