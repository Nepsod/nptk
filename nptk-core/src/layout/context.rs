// SPDX-License-Identifier: LGPL-3.0-only
use crate::layout::{Constraints, LayoutDirection, LayoutPhase};
use nalgebra::Vector2;
use taffy::{AvailableSpace, Rect, Size};

/// Viewport bounds represented as a rectangle (x, y, width, height).
#[derive(Debug, Clone, Copy)]
pub struct ViewportBounds {
    /// X position of viewport top-left corner
    pub x: f32,
    /// Y position of viewport top-left corner
    pub y: f32,
    /// Width of viewport
    pub width: f32,
    /// Height of viewport
    pub height: f32,
}

impl ViewportBounds {
    /// Create new viewport bounds
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if a point is within viewport
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Check if a rectangle intersects with viewport (with optional buffer)
    pub fn intersects(&self, x: f32, y: f32, width: f32, height: f32, buffer: f32) -> bool {
        let expanded_x = self.x - buffer;
        let expanded_y = self.y - buffer;
        let expanded_width = self.width + buffer * 2.0;
        let expanded_height = self.height + buffer * 2.0;
        
        x < expanded_x + expanded_width
            && x + width > expanded_x
            && y < expanded_y + expanded_height
            && y + height > expanded_y
    }
}

/// Context information passed to widgets during layout style computation.
///
/// This allows widgets to adapt their layout structure based on available
/// constraints, enabling responsive layouts that change based on container size.
#[derive(Debug, Clone)]
pub struct LayoutContext {
    /// The constraints from the parent layout.
    pub constraints: Constraints,
    /// The parent's computed size, if available.
    pub parent_size: Option<Vector2<f32>>,
    /// The available space in Taffy's format.
    pub available_space: Size<AvailableSpace>,
    /// The current layout phase (Measure or Layout).
    pub phase: LayoutPhase,
    /// The layout direction (LTR, RTL, or Auto).
    pub direction: LayoutDirection,
    /// The viewport bounds (visible area) for layout-level culling.
    pub viewport_bounds: Option<ViewportBounds>,
    /// The scroll offset for calculating visible ranges.
    pub scroll_offset: Option<Vector2<f32>>,
}

impl LayoutContext {
    /// Create a new LayoutContext with the given constraints.
    pub fn new(constraints: Constraints) -> Self {
        let available_space = Size {
            width: if constraints.max_width.is_finite() {
                AvailableSpace::Definite(constraints.max_width)
            } else {
                AvailableSpace::MaxContent
            },
            height: if constraints.max_height.is_finite() {
                AvailableSpace::Definite(constraints.max_height)
            } else {
                AvailableSpace::MaxContent
            },
        };

        Self {
            constraints,
            parent_size: None,
            available_space,
            phase: LayoutPhase::Layout, // Default to layout phase
            direction: LayoutDirection::Ltr, // Default to LTR
            viewport_bounds: None,
            scroll_offset: None,
        }
    }

    /// Create a LayoutContext with parent size information.
    pub fn with_parent_size(mut self, parent_size: Vector2<f32>) -> Self {
        self.parent_size = Some(parent_size);
        self
    }

    /// Create a LayoutContext with a specific phase.
    pub fn with_phase(mut self, phase: LayoutPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Create a LayoutContext with a specific direction.
    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Create a LayoutContext with viewport bounds for layout-level culling.
    pub fn with_viewport_bounds(mut self, viewport: ViewportBounds) -> Self {
        self.viewport_bounds = Some(viewport);
        self
    }

    /// Create a LayoutContext with scroll offset for calculating visible ranges.
    pub fn with_scroll_offset(mut self, offset: Vector2<f32>) -> Self {
        self.scroll_offset = Some(offset);
        self
    }

    /// Create an unbounded LayoutContext (no constraints).
    pub fn unbounded() -> Self {
        Self::new(Constraints::unbounded())
    }

    /// Create an unbounded LayoutContext for the measure phase.
    pub fn for_measure() -> Self {
        Self::unbounded().with_phase(LayoutPhase::Measure)
    }

    /// Create an unbounded LayoutContext for the layout phase.
    pub fn for_layout() -> Self {
        Self::unbounded().with_phase(LayoutPhase::Layout)
    }
}
