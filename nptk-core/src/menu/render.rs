//! Unified rendering system for menus
//!
//! Provides a single rendering implementation that works for both menubar dropdowns
//! and context menus.

use crate::app::font_ctx::FontContext;
use crate::menu::constants::*;
use crate::menu::theme::MenuThemeColors;
use crate::menu::unified::{MenuTemplate, MenuItem};
use crate::signal::Signal;
use crate::text_render::TextRenderContext;
use crate::vgi::Graphics;
use crate::vgi::shape_to_path;
use vello::kurbo::{Affine, Line, Point, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::{Brush, Color, Fill};

/// Menu geometry information for hit testing and layout
pub struct MenuGeometry {
    pub items: Vec<MenuItem>,
    pub rect: Rect,
}

impl MenuGeometry {
    pub fn new(
        template: &MenuTemplate,
        position: Point,
        text_render: &mut TextRenderContext,
        font_cx: &mut FontContext,
    ) -> Self {
        let items = template.items.clone();
        let (width, height) = calculate_menu_size(&items, text_render, font_cx);
        let rect = Rect::new(
            position.x,
            position.y,
            position.x + width,
            position.y + height,
        );
        Self { items, rect }
    }

    pub fn hit_test_index(&self, cursor: Point) -> Option<usize> {
        if !self.rect.contains(cursor) {
            return None;
        }
        let relative_y = cursor.y - self.rect.y0 - PADDING;
        if relative_y < 0.0 {
            return None;
        }
        let idx = (relative_y / ITEM_HEIGHT) as usize;
        if idx < self.items.len() {
            Some(idx)
        } else {
            None
        }
    }

    pub fn item_rect(&self, index: usize) -> Rect {
        let y = self.rect.y0 + PADDING + (index as f64 * ITEM_HEIGHT);
        Rect::new(self.rect.x0, y, self.rect.x1, y + ITEM_HEIGHT)
    }

    pub fn submenu_origin(&self, index: usize) -> Point {
        let item_top = self.rect.y0 + (index as f64 * ITEM_HEIGHT);
        // Position submenu directly adjacent to parent menu (no gap)
        Point::new(self.rect.x1, item_top)
    }
}

/// Calculate the size needed for a menu based on its items
pub fn calculate_menu_size(
    items: &[MenuItem],
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
) -> (f64, f64) {
    // Calculate height based on number of items
    let height = (items.len() as f64 * ITEM_HEIGHT) + PADDING * 2.0;

    // Calculate width based on longest item text + shortcut
    let mut max_total_width: f64 = MIN_WIDTH;
    for item in items {
        if !item.is_separator() {
            // Use action label if bound, otherwise item label
            let label = if let Some(ref action) = item.bound_action {
                action.text.get().clone()
            } else {
                item.label.clone()
            };

            // Measure actual text width using text renderer
            let (text_width, _) = text_render.measure_text_layout(
                font_cx,
                &label,
                None,
                FONT_SIZE as f32,
                None,
            );
            let text_width = text_width as f64;

            // Calculate shortcut width if present
            // Priority: Action shortcut > Item shortcut
            let shortcut_opt = if let Some(ref action) = item.bound_action {
                action.shortcut.get().clone()
            } else {
                item.shortcut.clone()
            };

            let shortcut_width: f64 = if let Some(ref shortcut) = shortcut_opt {
                let (sw, _) = text_render.measure_text_layout(
                    font_cx,
                    shortcut,
                    None,
                    FONT_SIZE as f32,
                    None,
                );
                sw as f64
            } else {
                0.0
            };

            // For right-aligned shortcuts, we need space for:
            // - text width + left padding
            // - minimum gap between text and shortcut
            // - shortcut width + right padding
            // - space for checkmark/arrow if needed
            let mut total_width = text_width
                + TEXT_PADDING * 2.0
                + MIN_TEXT_SHORTCUT_GAP
                + shortcut_width
                + SHORTCUT_RIGHT_PADDING;


            // Add space for submenu arrow or checkmark
            let is_checked = if let Some(ref action) = item.bound_action {
                *action.checked.get()
            } else {
                item.checked
            };
            
            if item.has_submenu() || is_checked {
                total_width += crate::menu::constants::CHECKMARK_ARROW_WIDTH;
            }

            max_total_width = max_total_width.max(total_width);
        }
    }

    let width = max_total_width.min(MAX_WIDTH);
    (width, height)
}

/// Render the menu background (shadow, background, border)
///
/// Draws the drop shadow, rounded background, and border for the menu.
/// This is called once per menu before rendering items.
fn render_menu_background(
    graphics: &mut dyn Graphics,
    rect: Rect,
    bg_color: Color,
    border_color: Color,
) {
    // Shadow
    let shadow_rect = RoundedRect::new(
        rect.x0 + 2.0,
        rect.y0 + 2.0,
        rect.x1 + 2.0,
        rect.y1 + 2.0,
        RoundedRectRadii::new(BORDER_RADIUS, BORDER_RADIUS, BORDER_RADIUS, BORDER_RADIUS),
    );
    graphics.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(Color::new([0.0, 0.0, 0.0, 0.2])),
        None,
        &shape_to_path(&shadow_rect),
    );

    // Main background
    let rounded_rect = RoundedRect::new(
        rect.x0,
        rect.y0,
        rect.x1,
        rect.y1,
        RoundedRectRadii::new(BORDER_RADIUS, BORDER_RADIUS, BORDER_RADIUS, BORDER_RADIUS),
    );
    graphics.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(bg_color),
        None,
        &shape_to_path(&rounded_rect),
    );
    graphics.stroke(
        &vello::kurbo::Stroke::new(1.0),
        Affine::IDENTITY,
        &Brush::Solid(border_color),
        None,
        &shape_to_path(&rounded_rect),
    );
}

/// Render a separator line
fn render_separator(
    graphics: &mut dyn Graphics,
    rect: Rect,
    y: f64,
    border_color: Color,
) {
    let sep_y = y + ITEM_HEIGHT / 2.0;
    let line = Line::new(
        Point::new(rect.x0 + SEPARATOR_PADDING, sep_y),
        Point::new(rect.x1 - SEPARATOR_PADDING, sep_y),
    );
    graphics.stroke(
        &vello::kurbo::Stroke::new(1.0),
        Affine::IDENTITY,
        &Brush::Solid(border_color),
        None,
        &shape_to_path(&line),
    );
}

/// Render menu item content (checkmark, text, shortcut, submenu arrow)
///
/// This function handles all the visual elements of a menu item except for
/// the hover background, which is rendered separately in `render_menu_item`.
fn render_menu_item_content(
    graphics: &mut dyn Graphics,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
    item: &MenuItem,
    rect: Rect,
    current_y: f64,
    text_color: Color,
    disabled_color: Color,
) {
    let is_checked = if let Some(ref action) = item.bound_action {
        *action.checked.get()
    } else {
        item.checked
    };
    
    let is_enabled = if let Some(ref action) = item.bound_action {
        *action.enabled.get()
    } else {
        item.enabled
    };

    // Draw checkmark if checked
    if is_checked {
        let check_x = rect.x0 + ITEM_TEXT_X_OFFSET;
        let check_y = current_y + ITEM_HEIGHT / 2.0;
        let check_color = if is_enabled { text_color } else { disabled_color };
        graphics.stroke(
            &vello::kurbo::Stroke::new(1.5),
            Affine::IDENTITY,
            &Brush::Solid(check_color),
            None,
            &shape_to_path(&Line::new(
                Point::new(check_x, check_y),
                Point::new(check_x + 4.0, check_y + 4.0),
            )),
        );
        graphics.stroke(
            &vello::kurbo::Stroke::new(1.5),
            Affine::IDENTITY,
            &Brush::Solid(check_color),
            None,
            &shape_to_path(&Line::new(
                Point::new(check_x + 4.0, check_y + 4.0),
                Point::new(check_x + 10.0, check_y - 4.0),
            )),
        );
    }

    // Draw text
    let item_text_color = if is_enabled { text_color } else { disabled_color };
    let checkmark_offset = if is_checked { crate::menu::constants::CHECKMARK_ARROW_WIDTH } else { 0.0 };
    let text_x = rect.x0 + TEXT_PADDING + checkmark_offset;
    let text_y = current_y + crate::menu::constants::TEXT_Y_OFFSET;
    
    let label = if let Some(ref action) = item.bound_action {
        action.text.get().clone()
    } else {
        item.label.clone()
    };

    text_render.render_text(
        font_cx,
        graphics,
        &label,
        None,
        FONT_SIZE as f32,
        Brush::Solid(item_text_color),
        Affine::translate((text_x, text_y)),
        true,
        Some((rect.width() - TEXT_PADDING * 2.0 - crate::menu::constants::TEXT_RENDERING_RESERVE) as f32),
    );

    // Draw shortcut if present
    let shortcut_opt = if let Some(ref action) = item.bound_action {
        action.shortcut.get().clone()
    } else {
        item.shortcut.clone()
    };

    if let Some(ref shortcut) = shortcut_opt {
        let shortcut_x = rect.x1 - SHORTCUT_RIGHT_PADDING - (shortcut.len() as f64 * SHORTCUT_CHAR_WIDTH);
        text_render.render_text(
            font_cx,
            graphics,
            shortcut,
            None,
            FONT_SIZE as f32,
            Brush::Solid(item_text_color),
            Affine::translate((shortcut_x, text_y)),
            true,
            None,
        );
    }

    // Draw submenu arrow if present
    if item.has_submenu() {
        let arrow_x = rect.x1 - ARROW_X_OFFSET;
        let arrow_y = current_y + ITEM_HEIGHT / 2.0;
        let arrow_color = if is_enabled { text_color } else { disabled_color };

        graphics.stroke(
            &vello::kurbo::Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(arrow_color),
            None,
            &shape_to_path(&Line::new(
                Point::new(arrow_x - ARROW_SIZE, arrow_y - ARROW_SIZE),
                Point::new(arrow_x, arrow_y),
            )),
        );
        graphics.stroke(
            &vello::kurbo::Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(arrow_color),
            None,
            &shape_to_path(&Line::new(
                Point::new(arrow_x, arrow_y),
                Point::new(arrow_x - ARROW_SIZE, arrow_y + ARROW_SIZE),
            )),
        );
    }
}

/// Render a single menu item (hover background and content)
///
/// Handles both the hover background and delegating to `render_menu_item_content`
/// for the actual item visual elements. Separators are handled via `render_separator`.
fn render_menu_item(
    graphics: &mut dyn Graphics,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
    item: &MenuItem,
    item_rect: Rect,
    menu_rect: Rect,
    current_y: f64,
    is_hovered: bool,
    hovered_color: Color,
    text_color: Color,
    hovered_text_color: Color,
    disabled_color: Color,
    border_color: Color,
) {
    // Draw hover background
    let is_enabled = if let Some(ref action) = item.bound_action {
        *action.enabled.get()
    } else {
        item.enabled
    };

    if is_hovered && is_enabled && !item.is_separator() {
        let item_rounded = RoundedRect::new(
            item_rect.x0 + ITEM_BG_MARGIN,
            item_rect.y0,
            item_rect.x1 - ITEM_BG_MARGIN,
            item_rect.y1,
            RoundedRectRadii::new(ITEM_BORDER_RADIUS, ITEM_BORDER_RADIUS, ITEM_BORDER_RADIUS, ITEM_BORDER_RADIUS),
        );
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(hovered_color),
            None,
            &shape_to_path(&item_rounded),
        );
    }

    if item.is_separator() {
        render_separator(graphics, menu_rect, current_y, border_color);
    } else {
        // Use hovered_text_color when hovered and enabled
        let effective_text_color = if is_hovered && is_enabled {
            hovered_text_color
        } else {
            text_color
        };
        
        render_menu_item_content(
            graphics,
            text_render,
            font_cx,
            item,
            menu_rect,
            current_y,
            effective_text_color,
            disabled_color,
        );
    }
}

/// Render a menu template to graphics
pub fn render_menu(
    graphics: &mut dyn Graphics,
    template: &MenuTemplate,
    position: Point,
    palette: &crate::theme::Palette,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
    cursor_pos: Option<Point>,
    hovered_index: Option<usize>,
) -> Rect {
    let geometry = MenuGeometry::new(template, position, text_render, font_cx);
    let rect = geometry.rect;

    // Extract theme colors from palette
    let theme_colors = MenuThemeColors::extract_from_palette(palette);

    // Render background
    render_menu_background(
        graphics,
        rect,
        theme_colors.bg_color,
        theme_colors.border_color,
    );

    // Determine hovered item from cursor or parameter
    let hovered = hovered_index.or_else(|| cursor_pos.and_then(|cursor| geometry.hit_test_index(cursor)));

    // Draw items
    let mut current_y = rect.y0 + PADDING;

    for (i, item) in geometry.items.iter().enumerate() {
        let item_rect = geometry.item_rect(i);
        let is_hovered = Some(i) == hovered;

        render_menu_item(
            graphics,
            text_render,
            font_cx,
            item,
            item_rect,
            rect,
            current_y,
            is_hovered,
            theme_colors.hovered_color,
            theme_colors.text_color,
            theme_colors.hovered_text_color,
            theme_colors.disabled_color,
            theme_colors.border_color,
        );

        current_y += ITEM_HEIGHT;
    }

    rect
}
