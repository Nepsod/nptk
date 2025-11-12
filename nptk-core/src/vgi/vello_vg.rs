use crate::vgi::Graphics;
use vello::kurbo::{Affine, BezPath, Shape, Stroke};
use vello::peniko::{Brush, Fill};
use vello::Scene;

/// A Vello-based implementation of the [Graphics] trait.
pub struct VelloGraphics<'a> {
    scene: &'a mut Scene,
}

impl<'a> VelloGraphics<'a> {
    /// Create a new VelloGraphics from a Vello Scene reference.
    ///
    /// This is the primary constructor for VelloGraphics when working
    /// with a direct `vello::Scene` reference.
    pub fn new(scene: &'a mut Scene) -> Self {
        Self { scene }
    }

    /// Create a VelloGraphics from a unified Scene enum.
    ///
    /// This method extracts the Vello scene from the unified Scene enum
    /// and creates a VelloGraphics wrapper. Returns `None` if the scene
    /// is not a Vello scene (e.g., it's a Hybrid scene).
    ///
    /// # Returns
    /// * `Some(VelloGraphics)` if the scene is a Vello scene
    /// * `None` if the scene is a different backend (e.g., Hybrid)
    pub fn from_scene(scene: &'a mut super::Scene) -> Option<Self> {
        scene.as_vello_mut().map(|s| Self { scene: s })
    }

    /// Get a mutable reference to the underlying Scene.
    pub fn scene_mut(&mut self) -> &mut Scene {
        self.scene
    }

    /// Fill a shape using a concrete shape type (more efficient than BezPath conversion).
    ///
    /// This method allows direct use of concrete shapes (Rect, RoundedRect, etc.)
    /// without converting to BezPath first, which is more efficient.
    pub fn fill_shape(
        &mut self,
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.scene
            .fill(fill_rule, transform, brush, brush_transform, shape);
    }

    /// Stroke a shape using a concrete shape type (more efficient than BezPath conversion).
    ///
    /// This method allows direct use of concrete shapes (Rect, RoundedRect, etc.)
    /// without converting to BezPath first, which is more efficient.
    pub fn stroke_shape(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        self.scene
            .stroke(style, transform, brush, brush_transform, shape);
    }

    /// Push a layer using a concrete shape type (more efficient than BezPath conversion).
    ///
    /// This method allows direct use of concrete shapes (Rect, RoundedRect, etc.)
    /// without converting to BezPath first, which is more efficient.
    pub fn push_layer_shape(
        &mut self,
        mix: vello::peniko::Mix,
        alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    ) {
        self.scene.push_layer(mix, alpha, transform, shape);
    }
}

impl<'a> Graphics for VelloGraphics<'a> {
    fn fill(
        &mut self,
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    ) {
        self.scene
            .fill(fill_rule, transform, brush, brush_transform, shape);
    }

    fn stroke(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    ) {
        self.scene
            .stroke(style, transform, brush, brush_transform, shape);
    }

    fn append(&mut self, other: &vello::Scene, transform: Option<Affine>) {
        self.scene.append(other, transform);
    }

    fn push_layer(
        &mut self,
        mix: vello::peniko::Mix,
        alpha: f32,
        transform: Affine,
        shape: &BezPath,
    ) {
        self.scene.push_layer(mix, alpha, transform, shape);
    }

    fn pop_layer(&mut self) {
        self.scene.pop_layer();
    }

    fn as_scene_mut(&mut self) -> Option<&mut vello::Scene> {
        Some(self.scene)
    }
}

/// A type alias for the default graphics implementation.
pub type DefaultGraphics<'a> = VelloGraphics<'a>;
