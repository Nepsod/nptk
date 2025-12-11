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
//! - **Color/geometry primitives**: Shared helpers for consistent fallbacks
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

pub mod components;
pub mod context;
pub mod primitives;
mod state;

pub use components::button::{button_visual, ButtonVisual};
pub use components::checkbox::{checkbox_visual, CheckboxVisual};
pub use components::slider::{slider_visual, SliderVisual};
pub use components::text_input::{text_input_visual, TextInputVisual};
pub use context::{RenderBounds, RenderContext};
pub use primitives::{button_mapping, checkbox_mapping, themed_color_or};
pub use state::{CheckboxState, InteractionState, WidgetState};

use crate::id::WidgetId;
use crate::theme::Theme;
use vello::peniko::Color;

/// Trait for theme-based widget rendering.
/// This centralizes all widget drawing logic in the theme system.
pub trait ThemeRenderer {
    /// Get button color for the given state
    fn get_button_color(&self, id: WidgetId, state: WidgetState) -> Color;

    /// Get checkbox color for the given state and checkbox state
    fn get_checkbox_color(
        &self,
        id: WidgetId,
        state: WidgetState,
        checkbox_state: CheckboxState,
    ) -> Color;

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
        components::button::button_visual(self, &id, state).background
    }

    fn get_checkbox_color(
        &self,
        id: WidgetId,
        _state: WidgetState,
        checkbox_state: CheckboxState,
    ) -> Color {
        let mapping = checkbox_mapping(checkbox_state);
        themed_color_or(self, &id, mapping.property, mapping.fallback)
    }

    fn get_checkbox_border_color(&self, id: WidgetId, state: WidgetState) -> Color {
        components::checkbox::checkbox_visual(self, &id, state, CheckboxState::Unchecked).border
    }

    fn get_text_input_color(&self, id: WidgetId, state: WidgetState) -> Color {
        components::text_input::text_input_visual(self, &id, state).background
    }

    fn get_text_input_border_color(&self, id: WidgetId, state: WidgetState) -> Color {
        components::text_input::text_input_visual(self, &id, state).border
    }

    fn get_slider_track_color(&self, id: WidgetId, state: WidgetState) -> Color {
        components::slider::slider_visual(self, &id, state).track
    }

    fn get_slider_thumb_color(&self, id: WidgetId, state: WidgetState) -> Color {
        components::slider::slider_visual(self, &id, state).thumb
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{properties::ThemeProperty, theme::dark::DarkTheme};

    #[test]
    fn button_mapping_matches_state() {
        assert_eq!(
            button_mapping(WidgetState::Pressed).property,
            ThemeProperty::ColorPressed
        );
        assert_eq!(
            button_mapping(WidgetState::Disabled).property,
            ThemeProperty::ColorDisabled
        );
    }

    #[test]
    fn checkbox_mapping_matches_state() {
        assert_eq!(
            checkbox_mapping(CheckboxState::Checked).property,
            ThemeProperty::ColorChecked
        );
    }

    #[test]
    fn theme_renderer_uses_palette_when_available() {
        let theme = DarkTheme::new();
        let button_id = WidgetId::new("nptk-widgets", "Button");
        let color = theme.get_button_color(button_id, WidgetState::Normal);
        assert_eq!(color, Color::from_rgb8(100, 150, 255));
    }

    #[test]
    fn slider_visual_prefers_custom_properties() {
        let theme = DarkTheme::new();
        let slider_id = WidgetId::new("nptk-widgets", "Slider");
        let visual = slider_visual(&theme, &slider_id, WidgetState::Normal);
        assert_eq!(visual.track, Color::from_rgb8(80, 80, 80));
        assert_eq!(visual.thumb, Color::from_rgb8(100, 150, 255));
    }
}
