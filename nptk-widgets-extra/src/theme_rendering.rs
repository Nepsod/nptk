// SPDX-License-Identifier: LGPL-3.0-only
//! # Theme Rendering Bridge for Extra Widgets
//!
//! This module provides helper functions for using the theme rendering system
//! with the LGPL widgets in this crate.

use nptk_core::app::focus::FocusState;
use nptk_core::layout::LayoutNode;
use nptk_core::vg::kurbo::{
    Affine, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_theme::id::WidgetId;
use nptk_theme::rendering::WidgetState;
use nptk_theme::theme::Theme;

// Re-export generic helpers from nptk-widgets
pub use nptk_widgets::theme_rendering::{
    checkbox_state_to_theme_state,
    button_state_to_widget_state,
    render_button_with_theme,
    render_checkbox_with_theme,
    render_slider_with_theme,
    widget_state_from_states,
    ThemeTextRenderer,
};

/// Convert radio button state to unified widget state
pub fn radio_button_state_to_widget_state(
    radio_state: crate::radio_button::RadioButtonState,
    _focus_state: FocusState,
    is_focused: bool,
    is_selected: bool,
    disabled: bool,
) -> WidgetState {
    // Handle disabled state first
    if disabled {
        return WidgetState::Disabled;
    }

    match radio_state {
        crate::radio_button::RadioButtonState::Idle => {
            if is_selected {
                if is_focused {
                    WidgetState::Selected
                } else {
                    WidgetState::Selected
                }
            } else if is_focused {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            }
        },
        crate::radio_button::RadioButtonState::Hovered => {
            if is_selected {
                if is_focused {
                    WidgetState::SelectedHovered
                } else {
                    WidgetState::SelectedHovered
                }
            } else if is_focused {
                WidgetState::FocusedHovered
            } else {
                WidgetState::Hovered
            }
        },
        crate::radio_button::RadioButtonState::Pressed => {
            if is_selected {
                if is_focused {
                    WidgetState::SelectedPressed
                } else {
                    WidgetState::SelectedPressed
                }
            } else if is_focused {
                WidgetState::FocusedPressed
            } else {
                WidgetState::Pressed
            }
        },
    }
}

/// Helper for rendering text inputs using the theme system
pub fn render_text_input_with_theme(
    theme: &mut dyn Theme,
    widget_id: &WidgetId,
    is_focused: bool,
    is_disabled: bool,
    layout: &LayoutNode,
    graphics: &mut dyn Graphics,
) {
    let theme_state = if is_disabled {
        WidgetState::Disabled
    } else if is_focused {
        WidgetState::Focused
    } else {
        WidgetState::Normal
    };

    let bounds = Rect::new(
        layout.layout.location.x as f64,
        layout.layout.location.y as f64,
        (layout.layout.location.x + layout.layout.size.width) as f64,
        (layout.layout.location.y + layout.layout.size.height) as f64,
    );

    let input_rect = RoundedRect::from_rect(bounds, RoundedRectRadii::from_single_radius(4.0));

    // Get input colors from theme
    let fill_color = theme.get_text_input_color(widget_id.clone(), theme_state);
    let border_color = theme.get_text_input_border_color(widget_id.clone(), theme_state);

    // Fill the input background
    let fill_brush = Brush::Solid(fill_color);
    graphics.fill(
        Fill::NonZero,
        Affine::default(),
        &fill_brush,
        None,
        &input_rect.to_path(0.1),
    );

    // Draw the input border
    let border_brush = Brush::Solid(border_color);
    let border_stroke = Stroke::new(1.0);
    graphics.stroke(
        &border_stroke,
        Affine::default(),
        &border_brush,
        None,
        &input_rect.to_path(0.1),
    );

    // Draw focus indicator if focused
    if theme_state.is_focused() {
        let focus_color = theme.get_focus_color(widget_id.clone());
        let focus_brush = Brush::Solid(focus_color);
        let focus_stroke = Stroke::new(2.0);
        graphics.stroke(
            &focus_stroke,
            Affine::default(),
            &focus_brush,
            None,
            &input_rect.to_path(0.1),
        );
    }
}

/// Helper for rendering progress bars using the theme system
pub fn render_progress_with_theme(
    theme: &mut dyn Theme,
    widget_id: &WidgetId,
    value: f32,
    is_indeterminate: bool,
    animation_time: f32,
    layout: &LayoutNode,
    graphics: &mut dyn Graphics,
) {
    let width = layout.layout.size.width as f64;
    let height = layout.layout.size.height as f64;
    let x = layout.layout.location.x as f64;
    let y = layout.layout.location.y as f64;

    // Get theme colors using ThemeRenderer - use proper progress bar colors
    let background_color = theme
        .get_property(
            widget_id.clone(),
            &nptk_theme::properties::ThemeProperty::Color,
        )
        .unwrap_or_else(|| nptk_core::vg::peniko::Color::from_rgb8(220, 220, 220));
    let progress_color = theme
        .get_property(
            widget_id.clone(),
            &nptk_theme::properties::ThemeProperty::ColorProgress,
        )
        .unwrap_or_else(|| nptk_core::vg::peniko::Color::from_rgb8(100, 150, 255));
    let border_color = theme
        .get_property(
            widget_id.clone(),
            &nptk_theme::properties::ThemeProperty::ColorBorder,
        )
        .unwrap_or_else(|| nptk_core::vg::peniko::Color::from_rgb8(180, 180, 180));

    // Draw background
    let background_rect = RoundedRect::new(
        x,
        y,
        x + width,
        y + height,
        RoundedRectRadii::from_single_radius(height / 4.0),
    );

    graphics.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(background_color),
        None,
        &background_rect.to_path(0.1),
    );

    // Draw border
    graphics.stroke(
        &Stroke::new(1.0),
        Affine::IDENTITY,
        &Brush::Solid(border_color),
        None,
        &background_rect.to_path(0.1),
    );

    // Draw progress
    if is_indeterminate {
        // Indeterminate mode: ping-pong animated progress
        let progress_width = width * 0.3; // 30% of total width
        let available_width = width - progress_width;

        // Create ping-pong animation: 0.0 -> 1.0 -> 0.0 -> 1.0...
        let ping_pong_time = if animation_time <= 0.5 {
            animation_time * 2.0 // 0.0 -> 1.0
        } else {
            2.0 - (animation_time * 2.0) // 1.0 -> 0.0
        };

        let progress_x = x + (ping_pong_time as f64 * available_width);

        let progress_rect = RoundedRect::new(
            progress_x,
            y + 1.0, // Small margin from border
            progress_x + progress_width,
            y + height - 1.0,
            RoundedRectRadii::from_single_radius((height - 2.0) / 4.0),
        );

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(progress_color),
            None,
            &progress_rect.to_path(0.1),
        );
    } else {
        // Determinate progress
        let progress_width = width * value.clamp(0.0, 1.0) as f64;

        if progress_width > 0.0 {
            let progress_rect = RoundedRect::new(
                x + 1.0, // Small margin from border
                y + 1.0,
                x + progress_width - 1.0,
                y + height - 1.0,
                RoundedRectRadii::from_single_radius((height - 2.0) / 4.0),
            );

            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(progress_color),
                None,
                &progress_rect.to_path(0.1),
            );
        }
    }
}
