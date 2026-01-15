// SPDX-License-Identifier: LGPL-3.0-only
use crate::layout::{Constraints, LayoutDirection, LayoutPhase};
use nalgebra::Vector2;
use taffy::{AvailableSpace, Size};

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
