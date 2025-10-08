//! # Theme Rendering System
//!
//! This module provides centralized widget rendering functionality within the theme system.
//! It follows GNUstep's approach of centralizing drawing logic in the theme rather than
//! having each widget handle its own rendering.
//!
//! ## Overview
//!
//! The rendering system consists of:
//!
//! - **[ThemeRenderer]**: Core trait for theme-based widget rendering
//! - **[WidgetState]**: Enum for different widget states
//! - **[RenderContext]**: Context for rendering operations
//! - **Widget-specific rendering methods**: Centralized drawing logic
//!
//! ## Key Features
//!
//! - **Centralized Rendering**: All widget drawing logic in theme system
//! - **State-Aware**: Different rendering based on widget state
//! - **Consistent Styling**: Unified appearance across all widgets
//! - **Extensible**: Easy to add new widget types and states
//! - **Performance**: Efficient rendering with proper caching
//!
//! ## Usage Examples
//!
//! ### Basic Theme Rendering
//!
//! ```rust
//! use nptk_theme::rendering::{ThemeRenderer, WidgetState, RenderContext};
//! use nptk_theme::id::WidgetId;
//! use nptk_core::vg::Scene;
//! use nptk_core::layout::LayoutNode;
//!
//! // Render a button using theme
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let state = WidgetState::Hovered;
//! let context = RenderContext::new(layout_node, scene);
//!
//! theme.render_button(button_id, state, &context);
//! ```
//!
//! ### Custom Widget Rendering
//!
//! ```rust
//! impl ThemeRenderer for MyTheme {
//!     fn render_custom_widget(&self, id: WidgetId, state: WidgetState, context: &RenderContext) {
//!         // Custom rendering logic
//!         let color = self.get_color_for_state(id, state);
//!         context.fill_rect(context.bounds(), color);
//!     }
//! }
//! ```

use peniko::Color;
use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::Theme;

/// The unified state of a widget for rendering purposes.
/// This combines interaction state, focus state, and widget-specific states for comprehensive rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    /// Widget is in its normal, idle state
    Normal,
    /// Widget is being hovered over
    Hovered,
    /// Widget is being pressed/clicked
    Pressed,
    /// Widget is disabled
    Disabled,
    /// Widget is focused (via keyboard navigation)
    Focused,
    /// Widget is both focused and hovered
    FocusedHovered,
    /// Widget is both focused and pressed
    FocusedPressed,
    /// Widget is selected (for widgets that support selection)
    Selected,
    /// Widget is both selected and hovered
    SelectedHovered,
    /// Widget is both selected and pressed
    SelectedPressed,
    /// Widget is released (for buttons after click)
    Released,
    /// Widget is both focused and released
    FocusedReleased,
    /// Widget is both selected and released
    SelectedReleased,
}

impl WidgetState {
    /// Check if the widget is in a focused state
    pub fn is_focused(&self) -> bool {
        matches!(self, WidgetState::Focused | WidgetState::FocusedHovered | WidgetState::FocusedPressed | WidgetState::FocusedReleased)
    }

    /// Check if the widget is in a hovered state
    pub fn is_hovered(&self) -> bool {
        matches!(self, WidgetState::Hovered | WidgetState::FocusedHovered | WidgetState::SelectedHovered)
    }

    /// Check if the widget is in a pressed state
    pub fn is_pressed(&self) -> bool {
        matches!(self, WidgetState::Pressed | WidgetState::FocusedPressed | WidgetState::SelectedPressed)
    }

    /// Check if the widget is in a released state
    pub fn is_released(&self) -> bool {
        matches!(self, WidgetState::Released | WidgetState::FocusedReleased | WidgetState::SelectedReleased)
    }

    /// Check if the widget is in a selected state
    pub fn is_selected(&self) -> bool {
        matches!(self, WidgetState::Selected | WidgetState::SelectedHovered | WidgetState::SelectedPressed | WidgetState::SelectedReleased)
    }

    /// Check if the widget is disabled
    pub fn is_disabled(&self) -> bool {
        matches!(self, WidgetState::Disabled)
    }

}

/// The interaction state of a widget (simplified from current button states)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionState {
    /// Widget is idle (not being interacted with)
    Idle,
    /// Widget is being hovered over
    Hovered,
    /// Widget is being pressed/clicked
    Pressed,
    /// Widget is disabled
    Disabled,
}

/// Checkbox state for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxState {
    /// Unchecked state
    Unchecked,
    /// Checked state  
    Checked,
    /// Indeterminate state (partially selected, like in Windows file trees)
    Indeterminate,
}

/// Trait for theme-based widget rendering.
/// This centralizes all widget drawing logic in the theme system.
pub trait ThemeRenderer {
    /// Get button color for the given state
    fn get_button_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get checkbox color for the given state and checkbox state
    fn get_checkbox_color(&self, id: WidgetId, state: WidgetState, checkbox_state: CheckboxState) -> Color;

    /// Get checkbox border color
    fn get_checkbox_border_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get text input color
    fn get_text_input_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get text input border color
    fn get_text_input_border_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get slider track color
    fn get_slider_track_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get slider thumb color
    fn get_slider_thumb_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get focus color for widgets
    fn get_focus_color(&self, id: WidgetId) -> Color;

    /// Get checkbox symbol color
    fn get_checkbox_symbol_color(&self, id: WidgetId, checkbox_state: CheckboxState) -> Color;
}

/// Blanket implementation for all types that implement both Theme and ThemeRenderer
impl<T> ThemeRenderer for T
where
    T: Theme,
{
    fn get_button_color(&self, id: WidgetId, state: WidgetState) -> Color {
        match state {
            WidgetState::Normal => self.get_property(id, &ThemeProperty::ColorIdle).unwrap_or(Color::from_rgb8(200, 200, 200)),
            WidgetState::Hovered => self.get_property(id, &ThemeProperty::ColorHovered).unwrap_or(Color::from_rgb8(180, 180, 180)),
            WidgetState::Pressed => self.get_property(id, &ThemeProperty::ColorPressed).unwrap_or(Color::from_rgb8(160, 160, 160)),
            WidgetState::Released => self.get_property(id, &ThemeProperty::ColorHovered).unwrap_or(Color::from_rgb8(180, 180, 180)),
            WidgetState::Focused => self.get_property(id, &ThemeProperty::ColorFocused).unwrap_or(Color::from_rgb8(100, 150, 255)),
            WidgetState::FocusedHovered => self.get_property(id, &ThemeProperty::ColorFocused).unwrap_or(Color::from_rgb8(100, 150, 255)),
            WidgetState::FocusedPressed => self.get_property(id, &ThemeProperty::ColorPressed).unwrap_or(Color::from_rgb8(80, 130, 235)),
            WidgetState::FocusedReleased => self.get_property(id, &ThemeProperty::ColorFocused).unwrap_or(Color::from_rgb8(100, 150, 255)),
            WidgetState::Selected => self.get_property(id, &ThemeProperty::ColorSelection).unwrap_or(Color::from_rgb8(100, 150, 255)),
            WidgetState::SelectedHovered => self.get_property(id, &ThemeProperty::ColorSelection).unwrap_or(Color::from_rgb8(100, 150, 255)),
            WidgetState::SelectedPressed => self.get_property(id, &ThemeProperty::ColorPressed).unwrap_or(Color::from_rgb8(80, 130, 235)),
            WidgetState::SelectedReleased => self.get_property(id, &ThemeProperty::ColorSelection).unwrap_or(Color::from_rgb8(100, 150, 255)),
            WidgetState::Disabled => self.get_property(id, &ThemeProperty::ColorDisabled).unwrap_or(Color::from_rgb8(150, 150, 150)),
        }
    }

    fn get_checkbox_color(&self, id: WidgetId, _state: WidgetState, checkbox_state: CheckboxState) -> Color {
        match checkbox_state {
            CheckboxState::Unchecked => self.get_property(id, &ThemeProperty::ColorUnchecked).unwrap_or(Color::from_rgb8(255, 255, 255)),
            CheckboxState::Checked => self.get_property(id, &ThemeProperty::ColorChecked).unwrap_or(Color::from_rgb8(100, 150, 255)),
            CheckboxState::Indeterminate => self.get_property(id, &ThemeProperty::ColorIndeterminate).unwrap_or(Color::from_rgb8(150, 150, 150)),
        }
    }

    fn get_checkbox_border_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            Color::from_rgb8(150, 150, 150)
        } else {
            Color::from_rgb8(200, 200, 200)
        }
    }

    fn get_text_input_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            Color::from_rgb8(150, 150, 150)
        } else {
            Color::from_rgb8(255, 255, 255)
        }
    }

    fn get_text_input_border_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_focused() {
            Color::from_rgb8(100, 150, 255)
        } else {
            Color::from_rgb8(200, 200, 200)
        }
    }

    fn get_slider_track_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            Color::from_rgb8(150, 150, 150)
        } else {
            Color::from_rgb8(220, 220, 220)
        }
    }

    fn get_slider_thumb_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            Color::from_rgb8(150, 150, 150)
        } else if state.is_pressed() {
            Color::from_rgb8(100, 150, 255)
        } else {
            Color::from_rgb8(180, 180, 180)
        }
    }

    fn get_focus_color(&self, _id: WidgetId) -> Color {
        Color::from_rgb8(100, 150, 255) // Blue focus color
    }

    fn get_checkbox_symbol_color(&self, _id: WidgetId, checkbox_state: CheckboxState) -> Color {
        match checkbox_state {
            CheckboxState::Unchecked => Color::TRANSPARENT, // No symbol for unchecked state
            CheckboxState::Checked | CheckboxState::Indeterminate => Color::WHITE,
        }
    }
}

