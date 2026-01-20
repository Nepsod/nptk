// SPDX-License-Identifier: LGPL-3.0-only

//! Draw call batching for rendering optimization.
//!
//! This module provides infrastructure for batching draw operations with similar
//! state to reduce GPU overhead. Currently, this is a placeholder that can be
//! extended in the future with full batching support.

use crate::vgi::Graphics;
use vello::kurbo::{Affine, BezPath};
use vello::peniko::{Brush, Fill, Mix};
use vello::Scene;

/// Graphics wrapper that can batch draw operations before submitting to the underlying scene.
/// 
/// Currently, this acts as a pass-through but provides the infrastructure for future
/// batching optimization where operations with similar state can be grouped together.
pub struct BatchedGraphics<'a> {
    /// The underlying scene to write to
    scene: &'a mut Scene,
    /// Whether batching is enabled (currently disabled by default until fully implemented)
    batching_enabled: bool,
}

impl<'a> BatchedGraphics<'a> {
    /// Create a new batched graphics wrapper.
    pub fn new(scene: &'a mut Scene) -> Self {
        Self {
            scene,
            batching_enabled: false, // Disabled by default until full implementation
        }
    }

    /// Enable or disable batching (useful for debugging or future implementation).
    pub fn set_batching_enabled(&mut self, enabled: bool) {
        self.batching_enabled = enabled;
    }
}

impl<'a> Graphics for BatchedGraphics<'a> {
    fn fill(
        &mut self,
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    ) {
        // For now, pass through directly to scene
        // Future implementation would batch operations with similar state
        if self.batching_enabled {
            // TODO: Implement batching logic here
            // Group operations by: brush type, fill rule, blend mode
            // Flush batches when state changes or at end of frame
        }
        self.scene.fill(fill_rule, transform, brush, brush_transform, shape);
    }

    fn stroke(
        &mut self,
        style: &vello::kurbo::Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    ) {
        // Pass through directly - stroke batching is more complex
        self.scene.stroke(style, transform, brush, brush_transform, shape);
    }

    fn append(&mut self, other: &vello::Scene, transform: Option<Affine>) {
        // TODO: If batching enabled, flush batches before appending
        self.scene.append(other, transform);
    }

    fn push_layer(
        &mut self,
        mix: Mix,
        alpha: f32,
        transform: Affine,
        shape: &BezPath,
    ) {
        // TODO: If batching enabled, flush batches before pushing layer
        self.scene.push_layer(mix, alpha, transform, shape);
    }

    fn pop_layer(&mut self) {
        // TODO: If batching enabled, flush batches before popping layer
        self.scene.pop_layer();
    }

    fn as_scene_mut(&mut self) -> Option<&mut vello::Scene> {
        // TODO: If batching enabled, flush batches before giving access
        Some(self.scene)
    }
}
