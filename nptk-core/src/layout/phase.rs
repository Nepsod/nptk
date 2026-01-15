// SPDX-License-Identifier: LGPL-3.0-only

/// Represents the phase of layout computation.
///
/// Layout systems typically use a two-phase approach:
/// 1. **Measure**: Determine the intrinsic size of widgets without applying constraints
/// 2. **Layout**: Apply constraints and compute final positions and sizes
///
/// This enum allows widgets and layout systems to distinguish between these phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPhase {
    /// Phase 1: Measure content to determine intrinsic sizes.
    ///
    /// During this phase, widgets should measure their content without
    /// applying parent constraints. This allows the layout system to
    /// understand the natural size of widgets before distributing space.
    Measure,

    /// Phase 2: Apply constraints and compute final layout.
    ///
    /// During this phase, widgets receive constraints from their parent
    /// and compute their final size and position. This is the actual
    /// layout computation phase.
    Layout,
}

impl LayoutPhase {
    /// Check if this is the measure phase.
    pub fn is_measure(&self) -> bool {
        matches!(self, LayoutPhase::Measure)
    }

    /// Check if this is the layout phase.
    pub fn is_layout(&self) -> bool {
        matches!(self, LayoutPhase::Layout)
    }
}
