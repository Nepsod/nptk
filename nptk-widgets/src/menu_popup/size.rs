// SPDX-License-Identifier: MIT OR Apache-2.0

//! Size calculation for menu popup widget

use super::constants::*;
use crate::menu_popup::MenuBarItem;

/// Calculate the size needed for the popup based on items
pub fn calculate_popup_size(items: &[MenuBarItem]) -> (f64, f64) {
    // Calculate height based on number of items
    let height = (items.len() as f64 * ITEM_HEIGHT) + POPUP_PADDING;

    // Calculate width based on longest item text + shortcut
    let mut max_total_width: f64 = MIN_WIDTH;
    for item in items {
        if item.label != SEPARATOR_LABEL {
            // Skip separators
            // Calculate text width (rough estimate)
            let text_width: f64 = item.label.len() as f64 * TEXT_CHAR_WIDTH;

            // Calculate shortcut width if present
            let shortcut_width: f64 = if let Some(ref shortcut) = item.shortcut {
                shortcut.len() as f64 * SHORTCUT_CHAR_WIDTH
            } else {
                0.0
            };

            // For right-aligned shortcuts, we need space for:
            // - text width + left padding
            // - minimum gap between text and shortcut (to avoid overlap)
            // - shortcut width + right padding
            let total_width = text_width
                + TEXT_PADDING
                + MIN_TEXT_SHORTCUT_GAP
                + shortcut_width
                + SHORTCUT_RIGHT_PADDING;
            max_total_width = max_total_width.max(total_width);
        }
    }

    let width: f64 = max_total_width.min(MAX_WIDTH);

    (width, height)
}
