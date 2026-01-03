//! Unified rendering system for menus
//!
//! Provides a single rendering implementation that works for both menubar dropdowns
//! and context menus.

use crate::app::font_ctx::FontContext;
use crate::menu::unified::{MenuTemplate, MenuItem};
use crate::text_render::TextRenderContext;
use crate::vgi::Graphics;
use crate::vgi::shape_to_path;
use nptk_theme::id::WidgetId;
use nptk_theme::properties::ThemeProperty;
use nptk_theme::theme::Theme;
use vello::kurbo::{Affine, Line, Point, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::{Brush, Color, Fill};

// Rendering constants
const ITEM_HEIGHT: f64 = 24.0;
const PADDING: f64 = 4.0;
const TEXT_PADDING: f64 = 10.0;
const SHORTCUT_RIGHT_PADDING: f64 = 12.0;
const MIN_TEXT_SHORTCUT_GAP: f64 = 40.0;
const MIN_WIDTH: f64 = 120.0;
const MAX_WIDTH: f64 = 400.0;
const BORDER_RADIUS: f64 = 4.0;
const FONT_SIZE: f64 = 14.0;
const SEPARATOR_LABEL: &str = "---";
const TEXT_CHAR_WIDTH: f64 = 7.0; // Approximate character width
const SHORTCUT_CHAR_WIDTH: f64 = 8.0; // Slightly wider for monospace shortcuts
const ARROW_SIZE: f64 = 3.0;

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
        let item_top = self.rect.y0 + PADDING + (index as f64 * ITEM_HEIGHT);
        Point::new(self.rect.x1 + 8.0, item_top)
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
            // Measure actual text width using text renderer
            let (text_width, _) = text_render.measure_text_layout(
                font_cx,
                &item.label,
                None,
                FONT_SIZE as f32,
                None,
            );
            let text_width = text_width as f64;

            // Calculate shortcut width if present
            let shortcut_width: f64 = if let Some(ref shortcut) = item.shortcut {
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
            if item.has_submenu() || item.checked {
                total_width += 20.0;
            }

            max_total_width = max_total_width.max(total_width);
        }
    }

    let width = max_total_width.min(MAX_WIDTH);
    (width, height)
}

/// Render a menu template to graphics
pub fn render_menu(
    graphics: &mut dyn Graphics,
    template: &MenuTemplate,
    position: Point,
    theme: &mut dyn Theme,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
    cursor_pos: Option<Point>,
    hovered_index: Option<usize>,
) -> Rect {
    let geometry = MenuGeometry::new(template, position, text_render, font_cx);
    let rect = geometry.rect;

    let menu_id = WidgetId::new("nptk-widgets", "MenuPopup");

    // Extract theme colors
    let bg_color = theme
        .get_property(menu_id.clone(), &ThemeProperty::ColorBackground)
        .unwrap_or(Color::from_rgb8(255, 255, 255));
    let border_color = theme
        .get_property(menu_id.clone(), &ThemeProperty::ColorBorder)
        .unwrap_or(Color::from_rgb8(200, 200, 200));
    let text_color = theme
        .get_property(menu_id.clone(), &ThemeProperty::ColorText)
        .unwrap_or(Color::from_rgb8(0, 0, 0));
    let hovered_color = theme
        .get_property(menu_id.clone(), &ThemeProperty::ColorMenuHovered)
        .unwrap_or(Color::from_rgb8(230, 230, 230));
    let disabled_color = theme
        .get_property(menu_id.clone(), &ThemeProperty::ColorText)
        .map(|c| {
            // Make disabled color more transparent
            let components = c.components;
            let r = components[0] as u8;
            let g = components[1] as u8;
            let b = components[2] as u8;
            let alpha = ((components[3] as f32) * 0.5).clamp(0.0, 255.0) as u8;
            Color::from_rgba8(r, g, b, alpha)
        })
        .unwrap_or(Color::from_rgb8(128, 128, 128));

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

    // Determine hovered item from cursor or parameter
    let hovered = hovered_index.or_else(|| cursor_pos.and_then(|cursor| geometry.hit_test_index(cursor)));

    // Draw items
    let mut current_y = rect.y0 + PADDING;

    for (i, item) in geometry.items.iter().enumerate() {
        let item_rect = geometry.item_rect(i);
        let is_hovered = Some(i) == hovered;

        // Draw hover background
        if is_hovered && item.enabled && !item.is_separator() {
            let item_rounded = RoundedRect::new(
                item_rect.x0 + 2.0,
                item_rect.y0,
                item_rect.x1 - 2.0,
                item_rect.y1,
                RoundedRectRadii::new(2.0, 2.0, 2.0, 2.0),
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
            // Draw separator
            let sep_y = current_y + ITEM_HEIGHT / 2.0;
            let line = Line::new(
                Point::new(rect.x0 + 8.0, sep_y),
                Point::new(rect.x1 - 8.0, sep_y),
            );
            graphics.stroke(
                &vello::kurbo::Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(border_color),
                None,
                &shape_to_path(&line),
            );
        } else {
            // Draw checkmark if checked
            if item.checked {
                let check_x = rect.x0 + 8.0;
                let check_y = current_y + ITEM_HEIGHT / 2.0;
                // Simple checkmark (can be improved)
                let check_color = if item.enabled { text_color } else { disabled_color };
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
            let item_text_color = if item.enabled { text_color } else { disabled_color };
            let text_x = rect.x0 + TEXT_PADDING + if item.checked { 20.0 } else { 0.0 };
            let text_y = current_y + 4.0;

            text_render.render_text(
                font_cx,
                graphics,
                &item.label,
                None,
                FONT_SIZE as f32,
                Brush::Solid(item_text_color),
                Affine::translate((text_x, text_y)),
                true,
                Some((rect.width() - TEXT_PADDING * 2.0 - 30.0) as f32),
            );

            // Draw shortcut if present
            if let Some(ref shortcut) = item.shortcut {
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
                let arrow_x = rect.x1 - 12.0;
                let arrow_y = current_y + ITEM_HEIGHT / 2.0;
                let arrow_color = if item.enabled { text_color } else { disabled_color };

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

        current_y += ITEM_HEIGHT;
    }

    rect
}
