// SPDX-License-Identifier: LGPL-3.0-only
//! # Theme Rendering Helpers for Extra Widgets
//!
//! This module provides helper functions for rendering widgets using the new role-based theming system.

use nptk_core::layout::LayoutNode;
use nptk_core::vg::kurbo::{
    Affine, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Fill};
use nptk_core::vgi::Graphics;

/// Helper for rendering progress bars using the palette
pub fn render_progress_with_theme(
    palette: &nptk_core::theme::Palette,
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

    // Get colors from palette
    let background_color = palette.color(nptk_core::theme::ColorRole::Base);
    let progress_color = palette.color(nptk_core::theme::ColorRole::Accent);
    let border_color = palette.color(nptk_core::theme::ColorRole::ThreedShadow1);

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
