use std::sync::{Arc, Mutex};
use vello::kurbo::Point;

/// A context menu containing a list of items.
#[derive(Clone)]
pub struct ContextMenu {
    /// Legacy flat items; treated as a single group when `groups` is None.
    pub items: Vec<ContextMenuItem>,
    /// Optional grouped items; when present, separators are auto-inserted between groups.
    pub groups: Option<Vec<ContextMenuGroup>>,
}

impl ContextMenu {
    pub fn new(items: Vec<ContextMenuItem>) -> Self {
        Self {
            items,
            groups: None,
        }
    }
}

#[derive(Clone)]
pub struct ContextMenuGroup {
    pub items: Vec<ContextMenuItem>,
}

/// An item in a context menu.
#[derive(Clone)]
pub enum ContextMenuItem {
    /// A clickable action item.
    Action {
        label: String,
        action: Arc<dyn Fn() + Send + Sync>,
    },
    /// A visual separator.
    Separator,
    /// A submenu.
    SubMenu {
        label: String,
        items: Vec<ContextMenuItem>,
    },
}

/// Result of a menu hit-test click.
pub enum MenuClickResult {
    Action(Arc<dyn Fn() + Send + Sync>),
    SubMenu(ContextMenu, Point),
    NonActionInside,
}

fn flatten_menu_items(menu: &ContextMenu) -> Vec<ContextMenuItem> {
    if let Some(groups) = &menu.groups {
        let mut out = Vec::new();
        for (i, group) in groups.iter().enumerate() {
            if i > 0 {
                out.push(ContextMenuItem::Separator);
            }
            out.extend(group.items.clone());
        }
        out
    } else {
        menu.items.clone()
    }
}

/// Manages the state of the active context menu.
#[derive(Clone, Default)]
pub struct ContextMenuManager {
    state: Arc<Mutex<ContextMenuState>>,
}

#[derive(Default)]
struct ContextMenuState {
    stack: Vec<(ContextMenu, Point)>,
}

impl ContextMenuManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ContextMenuState::default())),
        }
    }

    pub fn show_context_menu(&self, menu: ContextMenu, position: Point) {
        let mut state = self.state.lock().unwrap();
        state.stack.clear();
        state.stack.push((menu, position));
    }

    pub fn push_submenu(&self, menu: ContextMenu, position: Point) {
        let mut state = self.state.lock().unwrap();
        state.stack.push((menu, position));
    }

    pub fn close_context_menu(&self) {
        let mut state = self.state.lock().unwrap();
        state.stack.clear();
    }

    pub fn get_active_menu(&self) -> Option<(ContextMenu, Point)> {
        let state = self.state.lock().unwrap();
        state.stack.last().cloned()
    }

    pub fn get_menu_stack(&self) -> Vec<(ContextMenu, Point)> {
        let state = self.state.lock().unwrap();
        state.stack.clone()
    }

    pub fn is_open(&self) -> bool {
        !self.state.lock().unwrap().stack.is_empty()
    }
}

use vello::kurbo::{Affine, Rect};
use vello::peniko::{Brush, Color};
use nptk_theme::theme::Theme;
use crate::vgi::Graphics;

use crate::app::font_ctx::FontContext;
use crate::text_render::TextRenderContext;

use nptk_theme::properties::ThemeProperty;
use nptk_theme::id::WidgetId;
use crate::vgi::shape_to_path;
use vello::kurbo::{RoundedRect, RoundedRectRadii};

/// Renders the context menu.
/// Returns the bounds of the rendered menu for hit testing.
/// Renders the context menu.
/// Returns the bounds of the rendered menu for hit testing.
pub fn render_context_menu(
    graphics: &mut dyn Graphics,
    menu: &ContextMenu,
    position: Point,
    theme: &mut dyn Theme,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
    cursor_pos: Option<Point>,
) -> Rect {
    let flat_items = flatten_menu_items(menu);
    let (width, height) = calculate_layout_from_items(&flat_items, text_render, font_cx);
    let x = position.x as f64;
    let y = position.y as f64;
    let rect = Rect::new(x, y, x + width, y + height);

    let menu_id = WidgetId::new("nptk-widgets", "MenuPopup");

    // Colors
    let bg_color = theme.get_property(menu_id.clone(), &ThemeProperty::ColorBackground)
        .unwrap_or(Color::from_rgb8(255, 255, 255));
    let border_color = theme.get_property(menu_id.clone(), &ThemeProperty::ColorBorder)
        .unwrap_or(Color::from_rgb8(200, 200, 200));
    let text_color = theme.get_property(menu_id.clone(), &ThemeProperty::ColorText)
        .unwrap_or(Color::from_rgb8(0, 0, 0));
    let hovered_color = theme.get_property(menu_id.clone(), &ThemeProperty::ColorMenuHovered)
        .unwrap_or(Color::from_rgb8(230, 230, 230)); // Default hover color
    
    // Shadow
    let shadow_rect = RoundedRect::new(x + 2.0, y + 2.0, x + width + 2.0, y + height + 2.0, RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0));
    graphics.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(Color::new([0.0, 0.0, 0.0, 0.2])),
        None,
        &shape_to_path(&shadow_rect)
    );

    // Main background
    let rounded_rect = RoundedRect::new(x, y, x + width, y + height, RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0));
    graphics.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(bg_color),
        None,
        &shape_to_path(&rounded_rect)
    );
    graphics.stroke(
        &vello::kurbo::Stroke::new(1.0),
        Affine::IDENTITY,
        &Brush::Solid(border_color),
        None,
        &shape_to_path(&rounded_rect)
    );

    // Draw items
    let item_height = 24.0;
    let padding = 4.0;
    let mut current_y = y + padding;

    // Determine hovered item index
    let hovered_index = if let Some(cursor) = cursor_pos {
        if rect.contains(cursor) {
            let relative_y = cursor.y - y - padding;
            if relative_y >= 0.0 {
                let idx = (relative_y / item_height) as usize;
                if idx < flat_items.len() {
                    Some(idx)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    for (i, item) in flat_items.iter().enumerate() {
        let item_rect = Rect::new(x, current_y, x + width, current_y + item_height);
        
        // Draw hover background
        if Some(i) == hovered_index {
             match item {
                ContextMenuItem::Separator => {}, // Don't highlight separators
                _ => {
                    let item_rounded = RoundedRect::new(
                        item_rect.x0 + 2.0,
                        item_rect.y0,
                        item_rect.x1 - 2.0,
                        item_rect.y1,
                        RoundedRectRadii::new(2.0, 2.0, 2.0, 2.0),
                    );
                    graphics.fill(
                        vello::peniko::Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(hovered_color),
                        None,
                        &shape_to_path(&item_rounded)
                    );
                }
             }
        }

        match item {
            ContextMenuItem::Action { label, .. } => {
                // Render text
                text_render.render_text(
                    font_cx,
                    graphics,
                    label,
                    None,
                    14.0,
                    Brush::Solid(text_color),
                    Affine::translate((x + 10.0, current_y + 4.0)), // Top-left of text box
                    true,
                    Some(width as f32 - 20.0),
                );
            }
            ContextMenuItem::Separator => {
                let sep_y = current_y + item_height / 2.0;
                let line = vello::kurbo::Line::new(
                    (x + 8.0, sep_y),
                    (x + width - 8.0, sep_y)
                );
                graphics.stroke(
                    &vello::kurbo::Stroke::new(1.0),
                    Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgb8(200, 200, 200)),
                    None,
                    &shape_to_path(&line)
                );
            }
            ContextMenuItem::SubMenu { label, .. } => {
                // Render label
                text_render.render_text(
                    font_cx,
                    graphics,
                    label,
                    None,
                    14.0,
                    Brush::Solid(text_color),
                    Affine::translate((x + 10.0, current_y + 4.0)),
                    true,
                    Some(width as f32 - 30.0),
                );
                
                // Draw arrow
                let arrow_x = x + width - 12.0;
                let arrow_y = current_y + (item_height / 2.0);
                let arrow_size = 3.0;
                let arrow_stroke = vello::kurbo::Stroke::new(1.0);
                
                graphics.stroke(
                    &arrow_stroke,
                    Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgb8(100, 100, 100)),
                    None,
                    &shape_to_path(&vello::kurbo::Line::new(
                        Point::new(arrow_x - arrow_size, arrow_y - arrow_size),
                        Point::new(arrow_x, arrow_y),
                    ))
                );
                graphics.stroke(
                    &arrow_stroke,
                    Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgb8(100, 100, 100)),
                    None,
                    &shape_to_path(&vello::kurbo::Line::new(
                        Point::new(arrow_x, arrow_y),
                        Point::new(arrow_x - arrow_size, arrow_y + arrow_size),
                    ))
                );
            }
        }
        current_y += item_height;
    }

    rect
}

fn calculate_layout_from_items(
    items: &[ContextMenuItem],
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
) -> (f64, f64) {
    let item_height = 24.0;
    let padding = 4.0;
    let min_width = 120.0;
    let max_width = 400.0;
    
    // Measure text using the active font context for accurate width.
    let mut max_text_width: f64 = 0.0;
    for item in items {
        if let ContextMenuItem::Action { label, .. } | ContextMenuItem::SubMenu { label, .. } = item {
            let (text_width, _) = text_render.measure_text_layout(font_cx, label, 14.0, None);
            max_text_width = max_text_width.max(text_width as f64);
        }
    }
    // Add padding and clamp.
    let estimated = (max_text_width + 40.0).max(min_width);
    let width = estimated.min(max_width);
    let height = items.len() as f64 * item_height + padding * 2.0;
    (width, height)
}

pub fn get_menu_rect(
    menu: &ContextMenu,
    position: Point,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
) -> Rect {
    let flat_items = flatten_menu_items(menu);
    let (width, height) = calculate_layout_from_items(&flat_items, text_render, font_cx);
    let x = position.x as f64;
    let y = position.y as f64;
    Rect::new(x, y, x + width, y + height)
}

pub fn handle_click(
    menu: &ContextMenu,
    position: Point,
    cursor: Point,
    text_render: &mut TextRenderContext,
    font_cx: &mut FontContext,
) -> Option<MenuClickResult> {
    let flat_items = flatten_menu_items(menu);
    let rect = {
        let (width, height) = calculate_layout_from_items(&flat_items, text_render, font_cx);
        let x = position.x as f64;
        let y = position.y as f64;
        Rect::new(x, y, x + width, y + height)
    };
    if !rect.contains(cursor) {
        return None;
    }

    let item_height = 24.0;
    let padding = 4.0;
    let relative_y = cursor.y - position.y - padding;
    
    if relative_y < 0.0 {
        return None;
    }

    let index = (relative_y / item_height) as usize;
    if index >= flat_items.len() {
        return None;
    }

    // Compute item rect to position submenu (if any).
    let item_height = 24.0;
    let padding = 4.0;
    let item_top = position.y as f64 + padding + (index as f64 * item_height);
    let item_bottom = item_top + item_height;
    let submenu_origin = Point::new(
        rect.x1 as f64 + 8.0,
        item_top,
    );

    match &flat_items[index] {
        ContextMenuItem::Action { action, .. } => Some(MenuClickResult::Action(action.clone())),
        ContextMenuItem::SubMenu { items, .. } => {
            Some(MenuClickResult::SubMenu(ContextMenu { items: items.clone(), groups: None }, submenu_origin))
        }
        _ => Some(MenuClickResult::NonActionInside),
    }
}
