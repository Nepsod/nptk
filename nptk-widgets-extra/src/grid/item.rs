// SPDX-License-Identifier: LGPL-3.0-only
use nptk_core::layout::{Dimension, LengthPercentage};

/// Defines how a grid item should be sized.
///
/// This enum provides different sizing strategies for grid columns and rows,
/// similar to CSS Grid's track sizing functions.
#[derive(Debug, Clone, PartialEq)]
pub enum GridItem {
    /// Fixed size in points.
    Fixed(f32),

    /// Flexible size with minimum and flex factor.
    ///
    /// The item will grow to fill available space, but never shrink below `min`.
    /// The `flex` value determines how much space it gets relative to other flexible items.
    Flexible { min: f32, flex: f32 },

    /// Adaptive size that adjusts based on available space.
    ///
    /// The item will take up as much space as needed, with a minimum of `min`.
    /// Similar to CSS `auto`, it sizes based on content.
    Adaptive { min: f32 },
}

impl GridItem {
    /// Convert this grid item to a Taffy dimension for use in grid templates.
    pub fn to_dimension(&self) -> Dimension {
        match self {
            GridItem::Fixed(size) => Dimension::length(*size),
            GridItem::Flexible { min, flex: _ } => Dimension::length(*min), // Min size, flex handled separately
            GridItem::Adaptive { min } => Dimension::length(*min), // Min size, auto for max
        }
    }

    /// Get the flex grow value for this item.
    pub fn flex_grow(&self) -> f32 {
        match self {
            GridItem::Fixed(_) => 0.0,
            GridItem::Flexible { flex, .. } => *flex,
            GridItem::Adaptive { .. } => 1.0, // Adaptive items grow to fill space
        }
    }

    /// Get the minimum size for this item.
    pub fn min_size(&self) -> f32 {
        match self {
            GridItem::Fixed(size) => *size,
            GridItem::Flexible { min, .. } => *min,
            GridItem::Adaptive { min } => *min,
        }
    }
}
