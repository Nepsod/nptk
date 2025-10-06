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
use crate::theme::Theme;

/// The state of a widget for rendering purposes.
/// This combines interaction state with focus state for comprehensive rendering.
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
}

impl WidgetState {
    /// Check if the widget is in a focused state
    pub fn is_focused(&self) -> bool {
        matches!(self, WidgetState::Focused | WidgetState::FocusedHovered | WidgetState::FocusedPressed)
    }

    /// Check if the widget is in a hovered state
    pub fn is_hovered(&self) -> bool {
        matches!(self, WidgetState::Hovered | WidgetState::FocusedHovered | WidgetState::SelectedHovered)
    }

    /// Check if the widget is in a pressed state
    pub fn is_pressed(&self) -> bool {
        matches!(self, WidgetState::Pressed | WidgetState::FocusedPressed | WidgetState::SelectedPressed)
    }

    /// Check if the widget is in a selected state
    pub fn is_selected(&self) -> bool {
        matches!(self, WidgetState::Selected | WidgetState::SelectedHovered | WidgetState::SelectedPressed)
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
pub trait ThemeRenderer: Theme {
    /// Get button color for the given state
    fn get_button_color(&self, id: WidgetId, state: WidgetState) -> Color {
        if let Some(style) = self.of(id) {
            match state {
                WidgetState::Normal => style.get_color("color_idle").unwrap_or(self.defaults().interactive().inactive()),
                WidgetState::Hovered => style.get_color("color_hovered").unwrap_or(self.defaults().interactive().hover()),
                WidgetState::Pressed => style.get_color("color_pressed").unwrap_or(self.defaults().interactive().active()),
                WidgetState::Focused => style.get_color("color_focused").unwrap_or(self.defaults().interactive().hover()),
                WidgetState::FocusedHovered => style.get_color("color_focused").unwrap_or(self.defaults().interactive().hover()),
                WidgetState::FocusedPressed => style.get_color("color_pressed").unwrap_or(self.defaults().interactive().active()),
                WidgetState::Disabled => style.get_color("color_disabled").unwrap_or(self.defaults().interactive().disabled()),
                _ => self.defaults().interactive().inactive(),
            }
        } else {
            match state {
                WidgetState::Normal => self.defaults().interactive().inactive(),
                WidgetState::Hovered => self.defaults().interactive().hover(),
                WidgetState::Pressed => self.defaults().interactive().active(),
                WidgetState::Focused => self.defaults().interactive().hover(),
                WidgetState::FocusedHovered => self.defaults().interactive().hover(),
                WidgetState::FocusedPressed => self.defaults().interactive().active(),
                WidgetState::Disabled => self.defaults().interactive().disabled(),
                _ => self.defaults().interactive().inactive(),
            }
        }
    }

    /// Get checkbox color for the given state and checkbox state
    fn get_checkbox_color(&self, id: WidgetId, _state: WidgetState, checkbox_state: CheckboxState) -> Color {
        if let Some(style) = self.of(id) {
            match checkbox_state {
                CheckboxState::Unchecked => style.get_color("color_unchecked").unwrap_or(self.defaults().container().background()),
                CheckboxState::Checked => style.get_color("color_checked").unwrap_or(self.defaults().interactive().active()),
                CheckboxState::Indeterminate => style.get_color("color_indeterminate").unwrap_or(self.defaults().interactive().hover()),
            }
        } else {
            match checkbox_state {
                CheckboxState::Unchecked => self.defaults().container().background(),
                CheckboxState::Checked => self.defaults().interactive().active(),
                CheckboxState::Indeterminate => self.defaults().interactive().hover(),
            }
        }
    }

    /// Get checkbox border color
    fn get_checkbox_border_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            self.defaults().interactive().disabled()
        } else {
            Color::from_rgb8(200, 200, 200)
        }
    }

    /// Get text input color
    fn get_text_input_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            self.defaults().interactive().disabled()
        } else {
            self.defaults().container().background()
        }
    }

    /// Get text input border color
    fn get_text_input_border_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_focused() {
            self.defaults().interactive().active()
        } else {
            Color::from_rgb8(200, 200, 200)
        }
    }

    /// Get slider track color
    fn get_slider_track_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            self.defaults().interactive().disabled()
        } else {
            Color::from_rgb8(220, 220, 220)
        }
    }

    /// Get slider thumb color
    fn get_slider_thumb_color(&self, _id: WidgetId, state: WidgetState) -> Color {
        if state.is_disabled() {
            self.defaults().interactive().disabled()
        } else if state.is_pressed() {
            self.defaults().interactive().active()
        } else {
            self.defaults().interactive().hover()
        }
    }

    /// Get focus color for widgets
    fn get_focus_color(&self, _id: WidgetId) -> Color {
        Color::from_rgb8(100, 150, 255) // Blue focus color
    }

    /// Get checkbox symbol color
    fn get_checkbox_symbol_color(&self, _id: WidgetId, checkbox_state: CheckboxState) -> Color {
        match checkbox_state {
            CheckboxState::Unchecked => Color::TRANSPARENT, // No symbol for unchecked state
            CheckboxState::Checked | CheckboxState::Indeterminate => Color::WHITE,
        }
    }
}
