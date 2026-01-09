//! # Theme Rendering Bridge
//!
//! This module provides a bridge between the centralized theme rendering system
//! and the actual widget rendering. It converts widget states to theme states
//! and provides helper functions for using the theme rendering system.

use nptk_core::app::focus::FocusState;
use nptk_core::layout::LayoutNode;
use nptk_core::vg::kurbo::{
    Affine, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_theme::id::WidgetId;
use nptk_theme::rendering::{CheckboxState as ThemeCheckboxState, InteractionState, WidgetState};
use nptk_theme::theme::Theme;

/// Convert checkbox state to theme checkbox state
pub fn checkbox_state_to_theme_state(
    checkbox_state: crate::checkbox::CheckboxState,
) -> ThemeCheckboxState {
    match checkbox_state {
        crate::checkbox::CheckboxState::Unchecked => ThemeCheckboxState::Unchecked,
        crate::checkbox::CheckboxState::Checked => ThemeCheckboxState::Checked,
        crate::checkbox::CheckboxState::Indeterminate => ThemeCheckboxState::Indeterminate,
    }
}

/// Convert button state to unified widget state
pub fn button_state_to_widget_state(
    button_state: crate::button::ButtonState,
    _focus_state: FocusState,
    is_focused: bool,
    disabled: bool,
) -> WidgetState {
    // Handle disabled state first
    if disabled {
        return WidgetState::Disabled;
    }

    match button_state {
        crate::button::ButtonState::Idle => {
            if is_focused {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            }
        },
        crate::button::ButtonState::Hovered => {
            if is_focused {
                WidgetState::FocusedHovered
            } else {
                WidgetState::Hovered
            }
        },
        crate::button::ButtonState::Pressed => {
            if is_focused {
                WidgetState::FocusedPressed
            } else {
                WidgetState::Pressed
            }
        },
        crate::button::ButtonState::Released => {
            if is_focused {
                WidgetState::FocusedReleased
            } else {
                WidgetState::Released
            }
        },
    }
}

/// Helper for rendering buttons using the theme system
pub fn render_button_with_theme(
    theme: &mut dyn Theme,
    widget_id: &WidgetId,
    button_state: crate::button::ButtonState,
    focus_state: FocusState,
    is_focused: bool,
    disabled: bool,
    layout: &LayoutNode,
    graphics: &mut dyn Graphics,
) {
    let theme_state = button_state_to_widget_state(button_state, focus_state, is_focused, disabled);
    let bounds = Rect::new(
        layout.layout.location.x as f64,
        layout.layout.location.y as f64,
        (layout.layout.location.x + layout.layout.size.width) as f64,
        (layout.layout.location.y + layout.layout.size.height) as f64,
    );

    let rounded_rect = RoundedRect::from_rect(bounds, RoundedRectRadii::from_single_radius(10.0));

    // Get button colors from theme
    let fill_color = theme.get_button_color(widget_id.clone(), theme_state);
    let brush = Brush::Solid(fill_color);

    // Fill the button background
    graphics.fill(
        Fill::NonZero,
        Affine::default(),
        &brush,
        None,
        &rounded_rect.to_path(0.1),
    );

    // Draw focus indicator if focused
    if theme_state.is_focused() {
        let focus_color = theme.get_focus_color(widget_id.clone());
        let focus_brush = Brush::Solid(focus_color);
        let focus_stroke = Stroke::new(3.0);
        graphics.stroke(
            &focus_stroke,
            Affine::default(),
            &focus_brush,
            None,
            &rounded_rect.to_path(0.1),
        );
    }
}

/// Helper for rendering checkboxes using the theme system
pub fn render_checkbox_with_theme(
    theme: &mut dyn Theme,
    widget_id: &WidgetId,
    checkbox_state: crate::checkbox::CheckboxState,
    is_disabled: bool,
    layout: &LayoutNode,
    graphics: &mut dyn Graphics,
) {
    let theme_checkbox_state = checkbox_state_to_theme_state(checkbox_state);
    let theme_state = if is_disabled {
        WidgetState::Disabled
    } else {
        WidgetState::Normal
    };

    let bounds = Rect::new(
        layout.layout.location.x as f64,
        layout.layout.location.y as f64,
        (layout.layout.location.x + layout.layout.size.width) as f64,
        (layout.layout.location.y + layout.layout.size.height) as f64,
    );

    let checkbox_rect = RoundedRect::from_rect(bounds, RoundedRectRadii::from_single_radius(4.0));

    // Get checkbox colors from theme
    let fill_color = theme.get_checkbox_color(widget_id.clone(), theme_state, theme_checkbox_state);
    let border_color = theme.get_checkbox_border_color(widget_id.clone(), theme_state);

    // Fill the checkbox background
    let fill_brush = Brush::Solid(fill_color);
    graphics.fill(
        Fill::NonZero,
        Affine::default(),
        &fill_brush,
        None,
        &checkbox_rect.to_path(0.1),
    );

    // Draw the checkbox border
    let border_brush = Brush::Solid(border_color);
    let border_stroke = Stroke::new(1.0);
    graphics.stroke(
        &border_stroke,
        Affine::default(),
        &border_brush,
        None,
        &checkbox_rect.to_path(0.1),
    );

    // Draw the checkbox symbol (checkmark or indeterminate line)
    draw_checkbox_symbol_with_theme(theme, widget_id, theme_checkbox_state, bounds, graphics);
}

/// Draw checkbox symbol using theme colors
fn draw_checkbox_symbol_with_theme(
    theme: &dyn Theme,
    widget_id: &WidgetId,
    checkbox_state: ThemeCheckboxState,
    bounds: Rect,
    graphics: &mut dyn Graphics,
) {
    let symbol_color = theme.get_checkbox_symbol_color(widget_id.clone(), checkbox_state);

    if symbol_color == Color::TRANSPARENT {
        return; // No symbol for unchecked state
    }

    let symbol_brush = Brush::Solid(symbol_color);
    let symbol_stroke = Stroke::new(2.0);

    match checkbox_state {
        ThemeCheckboxState::Checked => {
            // Draw checkmark
            let checkmark_points = [
                Point::new(
                    bounds.x0 + bounds.width() * 0.2,
                    bounds.y0 + bounds.height() * 0.5,
                ),
                Point::new(
                    bounds.x0 + bounds.width() * 0.4,
                    bounds.y0 + bounds.height() * 0.7,
                ),
                Point::new(
                    bounds.x0 + bounds.width() * 0.8,
                    bounds.y0 + bounds.height() * 0.3,
                ),
            ];

            for i in 0..checkmark_points.len() - 1 {
                let start = checkmark_points[i];
                let end = checkmark_points[i + 1];
                let line = nptk_core::vg::kurbo::Line::new(start, end);
                graphics.stroke(
                    &symbol_stroke,
                    Affine::default(),
                    &symbol_brush,
                    None,
                    &line.to_path(0.1),
                );
            }
        },
        ThemeCheckboxState::Indeterminate => {
            // Draw horizontal line
            let line_y = bounds.y0 + bounds.height() * 0.5;
            let line = nptk_core::vg::kurbo::Line::new(
                Point::new(bounds.x0 + bounds.width() * 0.2, line_y),
                Point::new(bounds.x0 + bounds.width() * 0.8, line_y),
            );
            graphics.stroke(
                &symbol_stroke,
                Affine::default(),
                &symbol_brush,
                None,
                &line.to_path(0.1),
            );
        },
        ThemeCheckboxState::Unchecked => {
            // No symbol
        },
    }
}

/// Helper for rendering sliders using the theme system
pub fn render_slider_with_theme(
    theme: &mut dyn Theme,
    widget_id: &WidgetId,
    value: f32,
    is_disabled: bool,
    is_pressed: bool,
    layout: &LayoutNode,
    graphics: &mut dyn Graphics,
) {
    let theme_state = if is_disabled {
        WidgetState::Disabled
    } else if is_pressed {
        WidgetState::Pressed
    } else {
        WidgetState::Normal
    };

    let bounds = Rect::new(
        layout.layout.location.x as f64,
        layout.layout.location.y as f64,
        (layout.layout.location.x + layout.layout.size.width) as f64,
        (layout.layout.location.y + layout.layout.size.height) as f64,
    );

    // Draw slider track
    let track_color = theme.get_slider_track_color(widget_id.clone(), theme_state);
    let track_brush = Brush::Solid(track_color);
    let track_rect = RoundedRect::from_rect(bounds, RoundedRectRadii::from_single_radius(2.0));
    graphics.fill(
        Fill::NonZero,
        Affine::default(),
        &track_brush,
        None,
        &track_rect.to_path(0.1),
    );

    // Draw slider thumb
    let thumb_size = 16.0;
    let thumb_x = bounds.x0 + (bounds.width() * value as f64) - (thumb_size / 2.0);
    let thumb_y = bounds.y0 + (bounds.height() - thumb_size) / 2.0;
    let thumb_rect = Rect::new(thumb_x, thumb_y, thumb_x + thumb_size, thumb_y + thumb_size);
    let thumb_rounded = RoundedRect::from_rect(
        thumb_rect,
        RoundedRectRadii::from_single_radius(thumb_size / 2.0),
    );

    let thumb_color = theme.get_slider_thumb_color(widget_id.clone(), theme_state);
    let thumb_brush = Brush::Solid(thumb_color);
    graphics.fill(
        Fill::NonZero,
        Affine::default(),
        &thumb_brush,
        None,
        &thumb_rounded.to_path(0.1),
    );
}

/// Create a widget state from interaction state and focus state
pub fn widget_state_from_states(
    interaction_state: InteractionState,
    _focus_state: FocusState,
    is_focused: bool,
) -> WidgetState {
    let base_state = match interaction_state {
        InteractionState::Idle => {
            if is_focused {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            }
        },
        InteractionState::Hovered => {
            if is_focused {
                WidgetState::FocusedHovered
            } else {
                WidgetState::Hovered
            }
        },
        InteractionState::Pressed => {
            if is_focused {
                WidgetState::FocusedPressed
            } else {
                WidgetState::Pressed
            }
        },
        InteractionState::Disabled => WidgetState::Disabled,
    };

    // Note: FocusState::Selected doesn't exist in the current FocusState enum
    // This is a placeholder for future selection support
    base_state
}

/// Centralized text rendering helper for themes.
/// This provides a consistent way to render text across all widgets.
pub struct ThemeTextRenderer;

impl ThemeTextRenderer {
    /// Render text using theme colors and settings.
    ///
    /// # Arguments
    ///
    /// * `theme` - The theme to use for colors and settings
    /// * `widget_id` - The widget ID for theme property lookup
    /// * `text` - The text to render
    /// * `font_context` - The font context for rendering
    /// * `scene` - The scene to render to
    /// * `transform` - The transform to apply
    /// * `font_size` - The font size to use
    /// * `hinting` - Whether to use font hinting
    /// * `invert_color` - Whether to invert the text color
    ///
    /// # Returns
    ///
    /// The color that was used for rendering the text.
    pub fn render_text(
        theme: &dyn Theme,
        widget_id: WidgetId,
        text: &str,
        font_context: &mut nptk_core::app::font_ctx::FontContext,
        graphics: &mut dyn Graphics,
        transform: Affine,
        font_size: f32,
        hinting: bool,
        invert_color: bool,
    ) -> Color {
        let color = if invert_color {
            theme
                .get_property(
                    widget_id.clone(),
                    &nptk_theme::properties::ThemeProperty::ColorInvert,
                )
                .unwrap_or_else(|| Color::from_rgb8(0, 0, 0))
        } else {
            theme
                .get_property(
                    widget_id.clone(),
                    &nptk_theme::properties::ThemeProperty::Color,
                )
                .unwrap_or_else(|| Color::from_rgb8(0, 0, 0))
        };

        // Create a temporary TextRenderContext for rendering
        let mut text_render_context = nptk_core::text_render::TextRenderContext::new();

        text_render_context.render_text(
            font_context,
            graphics,
            text,
            None, // No specific font, use default
            font_size,
            Brush::Solid(color),
            transform,
            hinting,
            None, // No width constraint for theme rendering
        );

        color
    }

    /// Render text with placeholder support (for input widgets).
    ///
    /// # Arguments
    ///
    /// * `theme` - The theme to use for colors and settings
    /// * `widget_id` - The widget ID for theme property lookup
    /// * `text` - The text to render
    /// * `placeholder` - The placeholder text to show if text is empty
    /// * `font_context` - The font context for rendering
    /// * `scene` - The scene to render to
    /// * `transform` - The transform to apply
    /// * `font_size` - The font size to use
    /// * `hinting` - Whether to use font hinting
    ///
    /// # Returns
    ///
    /// The color that was used for rendering the text.
    pub fn render_text_with_placeholder(
        theme: &dyn Theme,
        widget_id: WidgetId,
        text: &str,
        placeholder: &str,
        font_context: &mut nptk_core::app::font_ctx::FontContext,
        graphics: &mut dyn Graphics,
        transform: Affine,
        font_size: f32,
        hinting: bool,
    ) -> Color {
        let (display_text, color) = if text.is_empty() {
            let placeholder_color = theme
                .get_property(
                    widget_id.clone(),
                    &nptk_theme::properties::ThemeProperty::ColorPlaceholder,
                )
                .unwrap_or_else(|| Color::from_rgb8(150, 150, 150));
            (placeholder, placeholder_color)
        } else {
            let text_color = theme
                .get_property(
                    widget_id.clone(),
                    &nptk_theme::properties::ThemeProperty::Color,
                )
                .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));
            (text, text_color)
        };

        // Create a temporary TextRenderContext for rendering
        let mut text_render_context = nptk_core::text_render::TextRenderContext::new();

        text_render_context.render_text(
            font_context,
            graphics,
            display_text,
            None, // No specific font, use default
            font_size,
            Brush::Solid(color),
            transform,
            hinting,
            None, // No width constraint for theme rendering
        );

        color
    }
}
