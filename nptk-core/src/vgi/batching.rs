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

/// Represents a batch of draw operations with the same state.
struct DrawBatch {
    fill_rule: Fill,
    transform: Affine,
    brush: Brush,
    path: BezPath,
    brush_transform: Option<Affine>,
}

/// Graphics wrapper that can batch draw operations before submitting to the underlying scene.
/// 
/// Currently, this acts as a pass-through but provides the infrastructure for future
/// batching optimization where operations with similar state can be grouped together.
pub struct BatchedGraphics<'a> {
    /// The underlying scene to write to
    scene: &'a mut Scene,
    /// Whether batching is enabled
    batching_enabled: bool,
    /// The current batch of operations
    current_batch: Option<DrawBatch>,
}

impl<'a> BatchedGraphics<'a> {
    /// Create a new batched graphics wrapper.
    pub fn new(scene: &'a mut Scene) -> Self {
        Self {
            scene,
            batching_enabled: true, // Enabled by default for solid brush optimization
            current_batch: None,
        }
    }

    /// Enable or disable batching (useful for debugging or future implementation).
    pub fn set_batching_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.flush_batch();
        }
        self.batching_enabled = enabled;
    }

    /// Flush the current batch to the scene.
    pub fn flush_batch(&mut self) {
        if let Some(batch) = self.current_batch.take() {
            self.scene.fill(
                batch.fill_rule,
                batch.transform,
                &batch.brush,
                batch.brush_transform,
                &batch.path,
            );
        }
    }
    /// Consumes the batched graphics and ensures the final batch is flushed.
    pub fn finish(mut self) {
        self.flush_batch();
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
        if self.batching_enabled {
            if let Brush::Solid(_) = brush {
                let can_batch = if let Some(batch) = &self.current_batch {
                    // Check if state is identical to current batch
                    batch.fill_rule == fill_rule
                        && batch.transform == transform
                        && batch.brush == *brush
                        && batch.brush_transform == brush_transform
                } else {
                    false
                };

                if can_batch {
                    // Append shape to current batch
                    if let Some(batch) = &mut self.current_batch {
                        batch.path.extend(shape.iter());
                    }
                    return;
                } else {
                    // Flush existing batch and start a new one
                    self.flush_batch();
                    self.current_batch = Some(DrawBatch {
                        fill_rule,
                        transform,
                        brush: brush.clone(),
                        brush_transform,
                        path: shape.clone(),
                    });
                    return;
                }
            } else {
                // Not a solid brush, flush and append directly
                self.flush_batch();
            }
        } else {
            self.flush_batch();
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
        self.flush_batch();
        self.scene.stroke(style, transform, brush, brush_transform, shape);
    }

    fn append(&mut self, other: &vello::Scene, transform: Option<Affine>) {
        self.flush_batch();
        self.scene.append(other, transform);
    }

    fn push_layer(
        &mut self,
        mix: Mix,
        alpha: f32,
        transform: Affine,
        shape: &BezPath,
    ) {
        self.flush_batch();
        self.scene.push_layer(mix, alpha, transform, shape);
    }

    fn pop_layer(&mut self) {
        self.flush_batch();
        self.scene.pop_layer();
    }

    fn as_scene_mut(&mut self) -> Option<&mut vello::Scene> {
        self.flush_batch();
        Some(self.scene)
    }
}
