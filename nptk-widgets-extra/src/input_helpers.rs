// SPDX-License-Identifier: LGPL-3.0-only
//! Shared helper functions for text input widgets.

use nptk_core::app::info::AppInfo;
use nptk_core::layout::LayoutNode;
use nptk_core::text_render::TextRenderContext;

/// Calculate cursor position from mouse coordinates using accurate text measurement.
pub fn cursor_position_from_mouse(
    text: &str,
    mouse_x: f32,
    layout_node: &LayoutNode,
    text_render_context: &TextRenderContext,
    info: &mut AppInfo,
) -> usize {
    let font_size = 16.0;
    let text_start_x = layout_node.layout.location.x + 8.0; // Padding
    let relative_x = mouse_x - text_start_x;

    if relative_x <= 0.0 {
        return 0;
    }

    // Find the character position by calculating cumulative text widths
    let mut current_width = 0.0;
    let mut char_position = 0;

    for (i, c) in text.chars().enumerate() {
        let char_text = c.to_string();
        let char_width = calculate_text_width(char_text.as_str(), font_size, text_render_context, info);

        if relative_x <= current_width + char_width / 2.0 {
            return i;
        }

        current_width += char_width;
        char_position = i + 1;
    }

    char_position
}

/// Calculate the actual width of text using Parley's font metrics.
pub fn calculate_text_width(
    text: &str,
    font_size: f32,
    text_render_context: &TextRenderContext,
    info: &mut AppInfo,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    // Use TextRenderContext to get accurate measurements from Parley
    // This handles all Unicode characters, emojis, and different scripts properly
    text_render_context
        .measure_text_width(&mut info.font_context, text, None, font_size)
}
