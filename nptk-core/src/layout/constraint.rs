// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;

/// Layout constraints that define the available space for a widget.
///
/// Similar to Flutter's `BoxConstraints`, this struct represents the
/// min/max width and height that a widget can use. Widgets must choose
/// their size within these constraints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    /// Minimum width the widget can be.
    pub min_width: f32,
    /// Maximum width the widget can be.
    pub max_width: f32,
    /// Minimum height the widget can be.
    pub min_height: f32,
    /// Maximum height the widget can be.
    pub max_height: f32,
}

impl Constraints {
    /// Create tight constraints (min == max, exact size required).
    ///
    /// The widget must be exactly this size.
    pub fn tight(size: Vector2<f32>) -> Self {
        Self {
            min_width: size.x,
            max_width: size.x,
            min_height: size.y,
            max_height: size.y,
        }
    }

    /// Create loose constraints (min == 0, max specified).
    ///
    /// The widget can be any size from 0 to max.
    pub fn loose(max: Vector2<f32>) -> Self {
        Self {
            min_width: 0.0,
            max_width: max.x,
            min_height: 0.0,
            max_height: max.y,
        }
    }

    /// Create unbounded constraints (no maximum limit).
    ///
    /// The widget can grow as large as it wants (used in scrollable containers).
    pub fn unbounded() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }

    /// Create constraints with specific min and max values.
    pub fn new(min: Vector2<f32>, max: Vector2<f32>) -> Self {
        Self {
            min_width: min.x,
            max_width: max.x,
            min_height: min.y,
            max_height: max.y,
        }
    }

    /// Check if these are tight constraints (min == max).
    pub fn is_tight(&self) -> bool {
        (self.min_width - self.max_width).abs() < f32::EPSILON
            && (self.min_height - self.max_height).abs() < f32::EPSILON
    }

    /// Check if width is unbounded (max_width is infinity).
    pub fn is_width_unbounded(&self) -> bool {
        self.max_width == f32::INFINITY
    }

    /// Check if height is unbounded (max_height is infinity).
    pub fn is_height_unbounded(&self) -> bool {
        self.max_height == f32::INFINITY
    }

    /// Constrain a size to fit within these constraints.
    pub fn constrain(&self, size: Vector2<f32>) -> Vector2<f32> {
        Vector2::new(
            size.x.max(self.min_width).min(self.max_width),
            size.y.max(self.min_height).min(self.max_height),
        )
    }

    /// Get the maximum size allowed by these constraints.
    pub fn max_size(&self) -> Vector2<f32> {
        Vector2::new(self.max_width, self.max_height)
    }

    /// Get the minimum size required by these constraints.
    pub fn min_size(&self) -> Vector2<f32> {
        Vector2::new(self.min_width, self.min_height)
    }
}

impl Default for Constraints {
    fn default() -> Self {
        Self::unbounded()
    }
}
