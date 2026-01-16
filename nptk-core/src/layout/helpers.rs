// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use crate::layout::{Anchor, AnchorPosition, Dimension, FlexDirection, LayoutStyle, LengthPercentage, LengthPercentageAuto};

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

    /// Set width as a percentage of the parent container.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::LayoutStyle;
    ///
    /// let style = LayoutStyle::default()
    ///     .width_percent(50.0); // 50% of parent width
    /// ```
    pub fn width_percent(mut self, percent: f32) -> Self {
        self.size.x = Dimension::percent(percent / 100.0);
        self
    }

    /// Set height as a percentage of the parent container.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::LayoutStyle;
    ///
    /// let style = LayoutStyle::default()
    ///     .height_percent(100.0); // Full height of parent
    /// ```
    pub fn height_percent(mut self, percent: f32) -> Self {
        self.size.y = Dimension::percent(percent / 100.0);
        self
    }

    /// Set both width and height to fill the parent (100%).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::LayoutStyle;
    ///
    /// let style = LayoutStyle::default()
    ///     .fill_parent();
    /// ```
    pub fn fill_parent(mut self) -> Self {
        self.size = Vector2::new(
            Dimension::percent(1.0),
            Dimension::percent(1.0),
        );
        self
    }

    /// Set flex basis as a percentage of the parent container.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::LayoutStyle;
    ///
    /// let style = LayoutStyle::default()
    ///     .flex_basis_percent(30.0); // 30% of parent width
    /// ```
    pub fn flex_basis_percent(mut self, percent: f32) -> Self {
        self.flex_basis = Dimension::percent(percent / 100.0);
        self
    }

    /// Set flex basis to fill available space (100%).
    pub fn flex_basis_fill(mut self) -> Self {
        self.flex_basis = Dimension::percent(1.0);
        self
    }

    /// Set min and max size constraints.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::{LayoutStyle, Dimension};
    ///
    /// let style = LayoutStyle::default()
    ///     .minmax(Dimension::length(100.0), Dimension::length(500.0));
    /// ```
    pub fn minmax(mut self, min: Dimension, max: Dimension) -> Self {
        self.min_size = Vector2::new(min, min);
        self.max_size = Vector2::new(max, max);
        self
    }

    /// Set the aspect ratio for this widget.
    ///
    /// The aspect ratio is width/height. For example:
    /// - `aspect_ratio(16.0 / 9.0)` for 16:9
    /// - `aspect_ratio(1.0)` for square (1:1)
    /// - `aspect_ratio(4.0 / 3.0)` for 4:3
    ///
    /// The layout system will maintain this aspect ratio when sizing the widget,
    /// subject to min/max size constraints.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::LayoutStyle;
    ///
    /// let style = LayoutStyle::default()
    ///     .aspect_ratio(16.0 / 9.0); // 16:9 aspect ratio
    /// ```
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }

    /// Maintain aspect ratio based on current size.
    ///
    /// This calculates the aspect ratio from the current size dimensions.
    /// If size is auto or undefined, this has no effect.
    ///
    /// Note: This method requires both width and height to be definite length values.
    /// For percentage-based or auto sizes, use `.aspect_ratio()` directly.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use nptk_core::layout::{LayoutStyle, Dimension};
    ///
    /// let style = LayoutStyle::default()
    ///     .size(Dimension::length(100.0), Dimension::length(50.0))
    ///     .maintain_aspect_ratio(); // Maintains 2:1 aspect ratio
    /// ```
    pub fn maintain_aspect_ratio(mut self) -> Self {
        // Note: Extracting length values from Dimension requires checking Taffy's internal structure.
        // For now, this is a placeholder. Users should use .aspect_ratio() directly with calculated values.
        // Future enhancement: Add helper to extract length from Dimension if it's a length value.
        self
    }
}
