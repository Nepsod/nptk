// SPDX-License-Identifier: MIT OR Apache-2.0

//! Rendering logic for menu popup widget

use nptk_core::app::font_ctx::FontContext;
use nptk_core::app::info::AppInfo;
use nptk_core::layout::LayoutNode;
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{
    Affine, Line, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_theme::theme::Theme;

use super::constants::*;
use super::layout::calculate_child_popup_layout;
use super::theme::ThemeColors;
use crate::menu_popup::MenuBarItem;
use crate::menu_popup::MenuPopup;

/// Render text helper
pub fn render_text(
    text_render_context: &mut TextRenderContext,
    font_cx: &mut FontContext,
    graphics: &mut dyn Graphics,
    text: &str,
    x: f64,
    y: f64,
    color: Color,
) {
    if text.is_empty() {
        return;
    }

    let transform = Affine::translate((x, y));

    // Try to render text, but don't panic if font context is not available
    let _ = text_render_context.render_text(
        font_cx,
        graphics,
        text,
        None, // No specific font, use default
        FONT_SIZE as f32,
        Brush::Solid(color),
        transform,
        true, // hinting
        None, // No width constraint for menu popup items
    );
}

/// Render the popup background and border
pub fn render_background_and_border(
    graphics: &mut dyn Graphics,
    popup_rect: Rect,
    colors: &ThemeColors,
) {
    let popup_rounded = RoundedRect::new(
        popup_rect.x0,
        popup_rect.y0,
        popup_rect.x1,
        popup_rect.y1,
        RoundedRectRadii::new(BORDER_RADIUS, BORDER_RADIUS, BORDER_RADIUS, BORDER_RADIUS),
    );

    // Draw background
    graphics.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(colors.bg_color),
        None,
        &popup_rounded.to_path(0.1),
    );

    // Draw border
    let stroke = Stroke::new(BORDER_STROKE_WIDTH);
    graphics.stroke(
        &stroke,
        Affine::IDENTITY,
        &Brush::Solid(colors.border_color),
        None,
        &popup_rounded.to_path(0.1),
    );
}

/// Render a single menu item
pub fn render_menu_item(
    graphics: &mut dyn Graphics,
    text_render_context: &mut TextRenderContext,
    font_cx: &mut FontContext,
    item: &MenuBarItem,
    item_rect: Rect,
    _item_index: usize,
    is_hovered: bool,
    colors: &ThemeColors,
) {
    // Determine item colors
    let (item_text_color, item_bg_color) = if !item.enabled {
        (colors.disabled_color, Color::TRANSPARENT)
    } else if is_hovered {
        (colors.text_color, colors.hovered_color)
    } else {
        (colors.text_color, Color::TRANSPARENT)
    };

    // Draw item background if needed
    if item_bg_color != Color::TRANSPARENT {
        let item_rounded = RoundedRect::new(
            item_rect.x0 + ITEM_BG_MARGIN,
            item_rect.y0,
            item_rect.x1 - ITEM_BG_MARGIN,
            item_rect.y1,
            RoundedRectRadii::new(
                ITEM_BORDER_RADIUS,
                ITEM_BORDER_RADIUS,
                ITEM_BORDER_RADIUS,
                ITEM_BORDER_RADIUS,
            ),
        );
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(item_bg_color),
            None,
            &item_rounded.to_path(0.1),
        );
    }

    // Draw item content
    if item.label != SEPARATOR_LABEL {
        // Draw item text
        let text_x = item_rect.x0 + ITEM_TEXT_X_OFFSET;
        let text_y = item_rect.y0 + ITEM_TEXT_Y_OFFSET;
        render_text(
            text_render_context,
            font_cx,
            graphics,
            &item.label,
            text_x,
            text_y,
            item_text_color,
        );

        // Draw keyboard shortcut if present
        if let Some(ref shortcut) = item.shortcut {
            // Calculate shortcut width
            let shortcut_width = shortcut.len() as f64 * SHORTCUT_CHAR_WIDTH;

            // Position shortcut at the right edge with padding
            let shortcut_x = item_rect.x1 - SHORTCUT_RIGHT_PADDING - shortcut_width;

            let shortcut_color = Color::from_rgb8(120, 120, 120); // Dimmed color
            render_text(
                text_render_context,
                font_cx,
                graphics,
                shortcut,
                shortcut_x,
                text_y,
                shortcut_color,
            );
        }

        // Draw submenu arrow if item has submenu
        if item.has_submenu() {
            let arrow_x = item_rect.x1 - ARROW_X_OFFSET;
            let arrow_y = item_rect.y0 + (ITEM_HEIGHT / 2.0);

            let arrow_stroke = Stroke::new(BORDER_STROKE_WIDTH);
            let arrow_color = if is_hovered {
                colors.text_color
            } else {
                Color::from_rgb8(100, 100, 100)
            };

            // Draw right-pointing arrow (> shape)
            graphics.stroke(
                &arrow_stroke,
                Affine::IDENTITY,
                &Brush::Solid(arrow_color),
                None,
                &Line::new(
                    Point::new(arrow_x - ARROW_SIZE, arrow_y - ARROW_SIZE),
                    Point::new(arrow_x, arrow_y),
                )
                .to_path(0.1),
            );
            graphics.stroke(
                &arrow_stroke,
                Affine::IDENTITY,
                &Brush::Solid(arrow_color),
                None,
                &Line::new(
                    Point::new(arrow_x, arrow_y),
                    Point::new(arrow_x - ARROW_SIZE, arrow_y + ARROW_SIZE),
                )
                .to_path(0.1),
            );
        }
    } else {
        // Draw separator line
        let sep_stroke = Stroke::new(BORDER_STROKE_WIDTH);
        let sep_y = item_rect.y0 + (ITEM_HEIGHT / 2.0);
        graphics.stroke(
            &sep_stroke,
            Affine::IDENTITY,
            &Brush::Solid(Color::from_rgb8(200, 200, 200)),
            None,
            &Line::new(
                Point::new(item_rect.x0 + SEPARATOR_PADDING, sep_y),
                Point::new(item_rect.x1 - SEPARATOR_PADDING, sep_y),
            )
            .to_path(0.1),
        );
    }
}

/// Render all menu items
pub fn render_menu_items(
    graphics: &mut dyn Graphics,
    text_render_context: &mut TextRenderContext,
    font_cx: &mut FontContext,
    items: &[MenuBarItem],
    popup_rect: Rect,
    hovered_index: Option<usize>,
    colors: &ThemeColors,
) {
    for (i, item) in items.iter().enumerate() {
        let item_y = popup_rect.y0 + ITEM_TOP_PADDING + (i as f64 * ITEM_HEIGHT);
        let item_rect = Rect::new(popup_rect.x0, item_y, popup_rect.x1, item_y + ITEM_HEIGHT);

        render_menu_item(
            graphics,
            text_render_context,
            font_cx,
            item,
            item_rect,
            i,
            hovered_index == Some(i),
            colors,
        );
    }
}

/// Calculate layout for child popup
pub fn calculate_child_popup_layout_for_render(
    parent_rect: Rect,
    open_index: usize,
    child_width: f64,
    child_height: f64,
) -> LayoutNode {
    calculate_child_popup_layout(
        parent_rect,
        open_index,
        child_width,
        child_height,
    )
}
