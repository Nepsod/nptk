// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use crate::layout::{Dimension, FlexDirection, LayoutStyle, LengthPercentage};

/// Helper methods for building LayoutStyle with a fluent API.
impl LayoutStyle {
    /// Create a quick flex configuration.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::LayoutStyle;
    ///
    /// let style = LayoutStyle::default()
    ///     .flex(1.0, 1.0, Dimension::auto());
    /// ```
    pub fn flex(mut self, grow: f32, shrink: f32, basis: Dimension) -> Self {
        self.flex_grow = grow;
        self.flex_shrink = shrink;
        self.flex_basis = basis;
        self
    }

    /// Set the size with a fluent API.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::{LayoutStyle, Dimension};
    ///
    /// let style = LayoutStyle::default()
    ///     .size(Dimension::length(100.0), Dimension::length(200.0));
    /// ```
    pub fn size(mut self, width: Dimension, height: Dimension) -> Self {
        self.size = Vector2::new(width, height);
        self
    }

    /// Set minimum size with a fluent API.
    pub fn min_size(mut self, width: Dimension, height: Dimension) -> Self {
        self.min_size = Vector2::new(width, height);
        self
    }

    /// Set maximum size with a fluent API.
    pub fn max_size(mut self, width: Dimension, height: Dimension) -> Self {
        self.max_size = Vector2::new(width, height);
        self
    }

    /// Set padding with a fluent API.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::{LayoutStyle, LengthPercentage};
    ///
    /// let style = LayoutStyle::default()
    ///     .padding(10.0);
    /// ```
    pub fn padding(mut self, padding: f32) -> Self {
        let p = LengthPercentage::length(padding);
        self.padding = taffy::Rect {
            left: p,
            right: p,
            top: p,
            bottom: p,
        };
        self
    }

    /// Set margin with a fluent API.
    pub fn margin(mut self, margin: f32) -> Self {
        use crate::layout::LengthPercentageAuto;
        let m = LengthPercentageAuto::length(margin);
        self.margin = taffy::Rect {
            left: m,
            right: m,
            top: m,
            bottom: m,
        };
        self
    }

    /// Set gap between children with a fluent API.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Vector2::new(
            LengthPercentage::length(gap),
            LengthPercentage::length(gap),
        );
        self
    }

    /// Set flex direction with a fluent API.
    pub fn flex_direction(mut self, direction: FlexDirection) -> Self {
        self.flex_direction = direction;
        self
    }

    /// Set layout priority with a fluent API.
    pub fn priority(mut self, priority: f32) -> Self {
        self.layout_priority = priority;
        self
    }
}
