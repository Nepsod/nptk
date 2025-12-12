// SPDX-License-Identifier: MIT OR Apache-2.0

//! Layout calculation helpers for menu popup widget

use nptk_core::layout::{Layout, LayoutNode};
use nptk_core::vg::kurbo::Rect;

use super::constants::*;

/// Calculate layout for a child popup positioned relative to a parent popup
pub fn calculate_child_popup_layout(
    parent_rect: Rect,
    item_index: usize,
    child_width: f64,
    child_height: f64,
) -> LayoutNode {
    let item_y = parent_rect.y0 + ITEM_TOP_PADDING + (item_index as f64 * ITEM_HEIGHT);

    // Position: right of parent popup, aligned with item top
    let child_x = parent_rect.x1 - CHILD_POPUP_OVERLAP; // Small overlap
    let child_y = item_y - ITEM_TOP_PADDING; // Align with item top (minus padding)

    let mut child_layout = LayoutNode {
        layout: Layout::default(),
        children: Vec::new(),
    };

    child_layout.layout.location.x = child_x as f32;
    child_layout.layout.location.y = child_y as f32;
    child_layout.layout.size.width = child_width as f32;
    child_layout.layout.size.height = child_height as f32;

    child_layout
}
