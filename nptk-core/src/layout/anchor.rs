// SPDX-License-Identifier: LGPL-3.0-only

use crate::layout::{LengthPercentageAuto, Position, Rect};

/// Anchor point for pinning widgets to specific positions.
///
/// Used with `Position::Absolute` to position widgets relative to their parent's edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Top center (horizontally centered at top)
    TopCenter,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    BottomRight,
    /// Bottom center (horizontally centered at bottom)
    BottomCenter,
    /// Left center (vertically centered on left)
    LeftCenter,
    /// Right center (vertically centered on right)
    RightCenter,
    /// Center of parent (both horizontally and vertically)
    Center,
}

/// Helper for converting anchor points to Taffy positioning.
///
/// This struct helps convert an `Anchor` enum value into the appropriate
/// `Position::Absolute` with `inset` values for Taffy layout.
pub struct AnchorPosition {
    /// The position type (always Absolute for anchors)
    pub position: Position,
    /// The inset values that position the widget relative to parent edges
    pub inset: Rect<LengthPercentageAuto>,
}

impl Anchor {
    /// Convert this anchor to positioning values with an optional offset.
    ///
    /// # Parameters
    ///
    /// - `offset`: Optional offset in pixels from the anchor point (default: 0.0)
    ///
    /// # Returns
    ///
    /// An `AnchorPosition` containing the `Position` and `inset` values needed
    /// to position a widget at this anchor point.
    pub fn to_position(&self, offset: f32) -> AnchorPosition {
        let offset_auto = LengthPercentageAuto::length(offset);
        let zero = LengthPercentageAuto::length(0.0);
        let auto = LengthPercentageAuto::auto();

        let (top, right, bottom, left) = match self {
            Anchor::TopLeft => (offset_auto, auto, auto, offset_auto),
            Anchor::TopRight => (offset_auto, offset_auto, auto, auto),
            Anchor::TopCenter => (offset_auto, auto, auto, auto),
            Anchor::BottomLeft => (auto, auto, offset_auto, offset_auto),
            Anchor::BottomRight => (auto, offset_auto, offset_auto, auto),
            Anchor::BottomCenter => (auto, auto, offset_auto, auto),
            Anchor::LeftCenter => (auto, auto, auto, offset_auto),
            Anchor::RightCenter => (auto, offset_auto, auto, auto),
            Anchor::Center => (auto, auto, auto, auto),
        };

        AnchorPosition {
            position: Position::Absolute,
            inset: Rect {
                top,
                right,
                bottom,
                left,
            },
        }
    }

    /// Convert this anchor to positioning values with separate horizontal and vertical offsets.
    ///
    /// # Parameters
    ///
    /// - `horizontal_offset`: Offset in pixels from the horizontal anchor point
    /// - `vertical_offset`: Offset in pixels from the vertical anchor point
    ///
    /// # Returns
    ///
    /// An `AnchorPosition` containing the `Position` and `inset` values.
    pub fn to_position_with_offsets(&self, horizontal_offset: f32, vertical_offset: f32) -> AnchorPosition {
        let h_offset = LengthPercentageAuto::length(horizontal_offset);
        let v_offset = LengthPercentageAuto::length(vertical_offset);
        let zero = LengthPercentageAuto::length(0.0);
        let auto = LengthPercentageAuto::auto();

        let (top, right, bottom, left) = match self {
            Anchor::TopLeft => (v_offset, auto, auto, h_offset),
            Anchor::TopRight => (v_offset, h_offset, auto, auto),
            Anchor::TopCenter => (v_offset, auto, auto, auto),
            Anchor::BottomLeft => (auto, auto, v_offset, h_offset),
            Anchor::BottomRight => (auto, h_offset, v_offset, auto),
            Anchor::BottomCenter => (auto, auto, v_offset, auto),
            Anchor::LeftCenter => (auto, auto, auto, h_offset),
            Anchor::RightCenter => (auto, h_offset, auto, auto),
            Anchor::Center => (auto, auto, auto, auto),
        };

        AnchorPosition {
            position: Position::Absolute,
            inset: Rect {
                top,
                right,
                bottom,
                left,
            },
        }
    }
}
