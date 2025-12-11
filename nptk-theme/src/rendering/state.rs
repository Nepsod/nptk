#![allow(missing_docs)]
use vello::peniko::Color;

/// The unified state of a widget for rendering purposes.
/// This combines interaction state, focus state, and widget-specific states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
    Focused,
    FocusedHovered,
    FocusedPressed,
    Selected,
    SelectedHovered,
    SelectedPressed,
    Released,
    FocusedReleased,
    SelectedReleased,
}

impl WidgetState {
    pub fn priority_color(&self) -> Color {
        match self {
            WidgetState::Normal => Color::from_rgb8(200, 200, 200),
            WidgetState::Hovered | WidgetState::FocusedHovered | WidgetState::SelectedHovered => {
                Color::from_rgb8(180, 180, 180)
            },
            WidgetState::Pressed | WidgetState::FocusedPressed | WidgetState::SelectedPressed => {
                Color::from_rgb8(160, 160, 160)
            },
            WidgetState::Released
            | WidgetState::FocusedReleased
            | WidgetState::SelectedReleased => Color::from_rgb8(180, 180, 180),
            WidgetState::Focused | WidgetState::Selected => Color::from_rgb8(100, 150, 255),
            WidgetState::Disabled => Color::from_rgb8(150, 150, 150),
        }
    }

    pub fn is_focused(&self) -> bool {
        matches!(
            self,
            WidgetState::Focused
                | WidgetState::FocusedHovered
                | WidgetState::FocusedPressed
                | WidgetState::FocusedReleased
        )
    }

    pub fn is_hovered(&self) -> bool {
        matches!(
            self,
            WidgetState::Hovered | WidgetState::FocusedHovered | WidgetState::SelectedHovered
        )
    }

    pub fn is_pressed(&self) -> bool {
        matches!(
            self,
            WidgetState::Pressed | WidgetState::FocusedPressed | WidgetState::SelectedPressed
        )
    }

    pub fn is_released(&self) -> bool {
        matches!(
            self,
            WidgetState::Released | WidgetState::FocusedReleased | WidgetState::SelectedReleased
        )
    }

    pub fn is_selected(&self) -> bool {
        matches!(
            self,
            WidgetState::Selected
                | WidgetState::SelectedHovered
                | WidgetState::SelectedPressed
                | WidgetState::SelectedReleased
        )
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, WidgetState::Disabled)
    }
}

/// The interaction state of a widget (simplified signal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionState {
    Idle,
    Hovered,
    Pressed,
    Disabled,
}

/// Checkbox state for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxState {
    Unchecked,
    Checked,
    Indeterminate,
}
