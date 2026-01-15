// SPDX-License-Identifier: LGPL-3.0-only

/// Text and layout direction for widgets.
///
/// This enum determines how content should be laid out and rendered,
/// particularly for right-to-left (RTL) languages like Arabic and Hebrew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    /// Left-to-right (LTR) layout direction.
    ///
    /// This is the default for most languages (English, Spanish, etc.).
    /// Content flows from left to right, and text is read left to right.
    Ltr,

    /// Right-to-left (RTL) layout direction.
    ///
    /// Used for languages like Arabic, Hebrew, and Urdu.
    /// Content flows from right to left, and text is read right to left.
    Rtl,

    /// Automatic direction detection.
    ///
    /// The direction is determined automatically based on the content
    /// or system locale. Currently defaults to LTR.
    Auto,
}

impl LayoutDirection {
    /// Check if this is RTL direction.
    pub fn is_rtl(&self) -> bool {
        matches!(self, LayoutDirection::Rtl)
    }

    /// Check if this is LTR direction.
    pub fn is_ltr(&self) -> bool {
        matches!(self, LayoutDirection::Ltr)
    }

    /// Get the resolved direction (Auto becomes Ltr for now).
    pub fn resolve(&self) -> LayoutDirection {
        match self {
            LayoutDirection::Auto => LayoutDirection::Ltr, // TODO: Detect from locale
            other => *other,
        }
    }

    /// Mirror an x-coordinate for RTL layouts.
    ///
    /// Given an x-coordinate and the parent width, returns the mirrored
    /// x-coordinate for RTL layouts. For LTR, returns the original x.
    ///
    /// # Parameters
    ///
    /// - `x`: The original x-coordinate
    /// - `width`: The width of the parent container
    ///
    /// # Returns
    ///
    /// The mirrored x-coordinate for RTL, or the original x for LTR.
    pub fn mirror_x(&self, x: f32, width: f32) -> f32 {
        if self.is_rtl() {
            width - x
        } else {
            x
        }
    }

    /// Mirror a layout position for RTL.
    ///
    /// Given a layout position (x, y) and the parent size, returns the
    /// mirrored position for RTL layouts.
    ///
    /// # Parameters
    ///
    /// - `x`: The original x-coordinate
    /// - `y`: The y-coordinate (unchanged)
    /// - `item_width`: The width of the item being positioned
    /// - `parent_width`: The width of the parent container
    ///
    /// # Returns
    ///
    /// The mirrored x-coordinate for RTL, or the original x for LTR.
    pub fn mirror_position(&self, x: f32, _y: f32, item_width: f32, parent_width: f32) -> f32 {
        if self.is_rtl() {
            parent_width - x - item_width
        } else {
            x
        }
    }
}

impl Default for LayoutDirection {
    fn default() -> Self {
        LayoutDirection::Ltr
    }
}
