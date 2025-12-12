// SPDX-License-Identifier: MIT OR Apache-2.0

//! Interaction logic for menu popup widget (hover, click, submenu management)

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::LayoutNode;
use nptk_core::vg::kurbo::{Point, Rect};
use nptk_core::window::{ElementState, MouseButton};
use std::sync::Arc;

use super::constants::*;
use super::layout::calculate_child_popup_layout;
use crate::menu_popup::{MenuBarItem, MenuPopup};

/// Detect which menu item is being hovered based on cursor position
pub fn detect_hovered_item(
    items: &[MenuBarItem],
    popup_rect: Rect,
    cursor_pos: Option<Point>,
) -> Option<usize> {
    if let Some(pos) = cursor_pos {
        // Check if mouse is within popup bounds
        if pos.x as f32 >= popup_rect.x0 as f32
            && pos.x as f32 <= popup_rect.x1 as f32
            && pos.y as f32 >= popup_rect.y0 as f32
            && pos.y as f32 <= popup_rect.y1 as f32
        {
            // Find which item is being hovered
            let relative_y = pos.y as f32 - popup_rect.y0 as f32 - ITEM_TOP_PADDING as f32;
            let item_index = (relative_y / ITEM_HEIGHT as f32) as usize;

            if item_index < items.len() {
                let item = &items[item_index];
                if item.enabled && item.label != SEPARATOR_LABEL {
                    return Some(item_index);
                }
            }
        }
    }
    None
}

/// Check if mouse is over a child popup
pub fn is_child_hovered(
    child_rect: Rect,
    cursor_pos: Option<Point>,
) -> bool {
    if let Some(pos) = cursor_pos {
        pos.x as f64 >= child_rect.x0
            && pos.x as f64 <= child_rect.x1
            && pos.y as f64 >= child_rect.y0
            && pos.y as f64 <= child_rect.y1
    } else {
        false
    }
}

/// Open a submenu for the given item
pub fn open_submenu(
    item: &MenuBarItem,
    on_close: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
) -> MenuPopup {
    let mut child = MenuPopup::new().with_items(item.submenu.clone());

    // Pass callbacks
    if let Some(ref cb) = on_close {
        let cb = cb.clone();
        child = child.with_on_close(move || cb());
    }

    child
}

/// Handle item click - execute callbacks and return update flags
pub fn handle_item_click(
    item: &MenuBarItem,
    item_index: usize,
    on_item_selected: Option<&Arc<dyn Fn(usize) -> Update + Send + Sync>>,
    on_close: Option<&Arc<dyn Fn() -> Update + Send + Sync>>,
) -> Update {
    let mut update = Update::empty();

    if item.enabled && item.label != SEPARATOR_LABEL {
        if !item.has_submenu() {
            // Execute item callback
            if let Some(ref callback) = item.on_activate {
                update |= callback();
            }

            // Notify parent of selection
            if let Some(ref callback) = on_item_selected {
                update |= callback(item_index);
            }

            // Close popup
            if let Some(ref callback) = on_close {
                update |= callback();
            }
        }
    }

    update
}

/// Handle click outside the popup
pub fn handle_click_outside(
    child_hovered: bool,
    on_close: Option<&Arc<dyn Fn() -> Update + Send + Sync>>,
) -> Update {
    let mut update = Update::empty();

    // Click outside - close popup ONLY if not in child
    if !child_hovered {
        if let Some(ref callback) = on_close {
            update |= callback();
        }
    }

    update
}

/// Calculate child popup layout and rect for update
pub fn calculate_child_popup_for_update(
    parent_rect: Rect,
    open_index: usize,
    child_width: f64,
    child_height: f64,
) -> (LayoutNode, Rect) {
    let child_layout = calculate_child_popup_layout(
        parent_rect,
        open_index,
        child_width,
        child_height,
    );

    let child_x = parent_rect.x1 - CHILD_POPUP_OVERLAP;
    let item_y = parent_rect.y0 + ITEM_TOP_PADDING + (open_index as f64 * ITEM_HEIGHT);
    let child_y = item_y - ITEM_TOP_PADDING;
    let child_rect = Rect::new(
        child_x,
        child_y,
        child_x + child_width,
        child_y + child_height,
    );

    (child_layout, child_rect)
}
