// SPDX-License-Identifier: LGPL-3.0-only

use nalgebra::Vector2;
use taffy::Layout;

/// Detects when content overflows its container bounds.
///
/// This helper is used to automatically detect overflow conditions
/// and trigger appropriate UI responses (e.g., showing scrollbars).
#[derive(Debug, Clone, Copy)]
pub struct OverflowDetector;

/// Represents overflow regions in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverflowRegions {
    /// Content overflows to the left of the container.
    pub left: bool,
    /// Content overflows to the right of the container.
    pub right: bool,
    /// Content overflows above the container.
    pub top: bool,
    /// Content overflows below the container.
    pub bottom: bool,
}

impl OverflowRegions {
    /// Create a new overflow regions struct with all flags set to false.
    pub fn new() -> Self {
        Self {
            left: false,
            right: false,
            top: false,
            bottom: false,
        }
    }

    /// Check if there is any overflow in any direction.
    pub fn has_overflow(&self) -> bool {
        self.left || self.right || self.top || self.bottom
    }

    /// Check if there is horizontal overflow (left or right).
    pub fn has_horizontal_overflow(&self) -> bool {
        self.left || self.right
    }

    /// Check if there is vertical overflow (top or bottom).
    pub fn has_vertical_overflow(&self) -> bool {
        self.top || self.bottom
    }
}

impl Default for OverflowRegions {
    fn default() -> Self {
        Self::new()
    }
}

impl OverflowDetector {
    /// Detect overflow by comparing content bounds to container bounds.
    ///
    /// # Parameters
    ///
    /// - `container_layout`: The layout of the container
    /// - `content_size`: The size of the content (may be larger than container)
    /// - `content_position`: The position of the content relative to container (for scrolling)
    ///
    /// # Returns
    ///
    /// An `OverflowRegions` struct indicating which directions have overflow.
    pub fn detect(
        container_layout: &Layout,
        content_size: Vector2<f32>,
        content_position: Vector2<f32>,
    ) -> OverflowRegions {
        let container_width = container_layout.size.width;
        let container_height = container_layout.size.height;

        // Content bounds relative to container
        let content_left = content_position.x;
        let content_right = content_position.x + content_size.x;
        let content_top = content_position.y;
        let content_bottom = content_position.y + content_size.y;

        // Container bounds (always 0,0 to width,height)
        let container_left = 0.0;
        let container_right = container_width;
        let container_top = 0.0;
        let container_bottom = container_height;

        OverflowRegions {
            left: content_left < container_left,
            right: content_right > container_right,
            top: content_top < container_top,
            bottom: content_bottom > container_bottom,
        }
    }

    /// Detect overflow for a child layout within a parent container.
    ///
    /// This is a convenience method that extracts sizes from Taffy layouts.
    ///
    /// # Parameters
    ///
    /// - `container_layout`: The layout of the container
    /// - `child_layout`: The layout of the child content
    ///
    /// # Returns
    ///
    /// An `OverflowRegions` struct indicating which directions have overflow.
    pub fn detect_from_layouts(container_layout: &Layout, child_layout: &Layout) -> OverflowRegions {
        let content_size = Vector2::new(child_layout.size.width, child_layout.size.height);
        let content_position = Vector2::new(child_layout.location.x, child_layout.location.y);

        Self::detect(container_layout, content_size, content_position)
    }

    /// Check if content size exceeds container size (simple overflow check).
    ///
    /// This is a simpler check that doesn't account for scrolling position.
    /// Use `detect()` for more accurate overflow detection with scrolling.
    ///
    /// # Parameters
    ///
    /// - `container_size`: The size of the container
    /// - `content_size`: The size of the content
    ///
    /// # Returns
    ///
    /// `true` if content is larger than container in any dimension.
    pub fn exceeds_bounds(container_size: Vector2<f32>, content_size: Vector2<f32>) -> bool {
        content_size.x > container_size.x || content_size.y > container_size.y
    }
}
