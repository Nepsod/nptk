//! Vector Graphics Interface abstraction.
//!
//! This module provides an abstraction over rendering backends, allowing widgets
//! to be decoupled from the specific rendering implementation (e.g., Vello).

use vello::kurbo::{Affine, BezPath, Shape, Stroke};
use vello::peniko::{Brush, Fill};

/// A trait for rendering vector graphics.
///
/// This trait abstracts over different rendering backends, allowing widgets to
/// be written without being tied to a specific implementation.
///
/// Note: Methods use `&BezPath` for object-safety. To use concrete shape types
/// (Rect, RoundedRect, Line, etc.), convert them to BezPath using `shape.to_path(0.1)`.
pub trait Graphics {
    /// Fill a shape with the given brush.
    fn fill(
        &mut self,
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    );

    /// Stroke a shape with the given brush.
    fn stroke(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    );

    /// Append another graphics scene to this one.
    ///
    /// Note: This method takes a Scene reference directly, since append
    /// operations typically work with Vello scenes. This maintains compatibility
    /// with existing widget code.
    fn append(&mut self, other: &vello::Scene, transform: Option<Affine>);

    /// Push a new layer with the given blend mode and transform.
    fn push_layer(&mut self, mix: vello::peniko::Mix, alpha: f32, transform: Affine, shape: &BezPath);

    /// Pop the most recent layer.
    fn pop_layer(&mut self);

    /// Access the underlying Scene for operations that require it (e.g., Parley text rendering).
    /// Returns None if the graphics backend doesn't provide Scene access.
    fn as_scene_mut(&mut self) -> Option<&mut vello::Scene>;
}

/// Helper function to convert a shape to BezPath for use with Graphics trait.
pub fn shape_to_path(shape: &impl Shape) -> BezPath {
    shape.to_path(0.1)
}

/// A default graphics implementation using Vello.
pub mod vello_vg;
