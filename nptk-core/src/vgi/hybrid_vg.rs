use crate::vgi::Graphics;
use vello::kurbo::{Affine, BezPath, Stroke};
use vello::peniko::{Brush, Fill};
use vello_common::kurbo::{
    Affine as VelloCommonAffine, BezPath as VelloCommonBezPath, Stroke as VelloCommonStroke,
};
use vello_common::paint::PaintType;
use vello_common::peniko::{BlendMode, Compose};
use vello_common::peniko::{Fill as VelloCommonFill, Mix as VelloCommonMix};

/// A Vello Hybrid-based implementation of the [Graphics] trait.
///
/// This implementation wraps `vello_hybrid::Scene` and provides the same
/// Graphics API as VelloGraphics, allowing widgets to work with either backend.
///
/// **Note:** vello_hybrid uses a stateful API, so this implementation
/// manages state internally and converts the stateless Graphics API calls
/// to stateful vello_hybrid API calls.
pub struct HybridGraphics<'a> {
    scene: &'a mut vello_hybrid::Scene,
    /// Current transform stack for managing nested transforms
    transform_stack: Vec<Affine>,
}

impl<'a> HybridGraphics<'a> {
    /// Create a new HybridGraphics from a Hybrid Scene reference.
    ///
    /// This is the primary constructor for HybridGraphics when working
    /// with a direct `vello_hybrid::Scene` reference.
    pub fn new(scene: &'a mut vello_hybrid::Scene) -> Self {
        Self {
            scene,
            transform_stack: vec![Affine::IDENTITY],
        }
    }

    /// Create a HybridGraphics from a unified Scene enum.
    ///
    /// This method extracts the Hybrid scene from the unified Scene enum
    /// and creates a HybridGraphics wrapper. Returns `None` if the scene
    /// is not a Hybrid scene (e.g., it's a Vello scene).
    ///
    /// # Returns
    /// * `Some(HybridGraphics)` if the scene is a Hybrid scene
    /// * `None` if the scene is a different backend (e.g., Vello)
    pub fn from_scene(scene: &'a mut super::Scene) -> Option<Self> {
        scene.as_hybrid_mut().map(|s| Self::new(s))
    }

    /// Get a mutable reference to the underlying Scene.
    pub fn scene_mut(&mut self) -> &mut vello_hybrid::Scene {
        self.scene
    }

    /// Convert vello::kurbo::Affine to vello_common::kurbo::Affine.
    fn convert_affine(&self, affine: Affine) -> VelloCommonAffine {
        // Affine is a 2x3 matrix, convert by copying coefficients
        VelloCommonAffine::new(affine.as_coeffs())
    }

    /// Convert vello::peniko::Fill to vello_common::peniko::Fill.
    fn convert_fill(&self, fill: Fill) -> VelloCommonFill {
        // Fill is an enum, convert by matching
        match fill {
            Fill::NonZero => VelloCommonFill::NonZero,
            Fill::EvenOdd => VelloCommonFill::EvenOdd,
        }
    }

    /// Convert vello::kurbo::Stroke to vello_common::kurbo::Stroke.
    fn convert_stroke(&self, stroke: &Stroke) -> VelloCommonStroke {
        // Stroke has multiple fields, convert by reconstructing
        use vello_common::kurbo::{Cap, Join};
        VelloCommonStroke {
            width: stroke.width,
            join: match stroke.join {
                vello::kurbo::Join::Miter => Join::Miter,
                vello::kurbo::Join::Round => Join::Round,
                vello::kurbo::Join::Bevel => Join::Bevel,
            },
            start_cap: match stroke.start_cap {
                vello::kurbo::Cap::Butt => Cap::Butt,
                vello::kurbo::Cap::Round => Cap::Round,
                vello::kurbo::Cap::Square => Cap::Square,
            },
            end_cap: match stroke.end_cap {
                vello::kurbo::Cap::Butt => Cap::Butt,
                vello::kurbo::Cap::Round => Cap::Round,
                vello::kurbo::Cap::Square => Cap::Square,
            },
            dash_pattern: stroke.dash_pattern.clone(),
            dash_offset: stroke.dash_offset,
            miter_limit: stroke.miter_limit,
        }
    }

    /// Convert vello::peniko::Mix to vello_common::peniko::Mix.
    fn convert_mix(&self, mix: vello::peniko::Mix) -> VelloCommonMix {
        // Mix is an enum, convert by matching all variants
        match mix {
            vello::peniko::Mix::Normal => VelloCommonMix::Normal,
            vello::peniko::Mix::Multiply => VelloCommonMix::Multiply,
            vello::peniko::Mix::Screen => VelloCommonMix::Screen,
            vello::peniko::Mix::Overlay => VelloCommonMix::Overlay,
            vello::peniko::Mix::Darken => VelloCommonMix::Darken,
            vello::peniko::Mix::Lighten => VelloCommonMix::Lighten,
            vello::peniko::Mix::ColorDodge => VelloCommonMix::ColorDodge,
            vello::peniko::Mix::ColorBurn => VelloCommonMix::ColorBurn,
            vello::peniko::Mix::HardLight => VelloCommonMix::HardLight,
            vello::peniko::Mix::SoftLight => VelloCommonMix::SoftLight,
            vello::peniko::Mix::Difference => VelloCommonMix::Difference,
            vello::peniko::Mix::Exclusion => VelloCommonMix::Exclusion,
            vello::peniko::Mix::Hue => VelloCommonMix::Hue,
            vello::peniko::Mix::Saturation => VelloCommonMix::Saturation,
            vello::peniko::Mix::Color => VelloCommonMix::Color,
            vello::peniko::Mix::Luminosity => VelloCommonMix::Luminosity,
            // Note: Mix::Clip is deprecated in vello_common, but we still need to handle it
            // for compatibility with vello::peniko::Mix::Clip
            #[allow(deprecated)]
            vello::peniko::Mix::Clip => VelloCommonMix::Clip,
        }
    }

    /// Convert vello::kurbo::BezPath to vello_common::kurbo::BezPath.
    fn convert_bezpath(&self, path: &BezPath) -> VelloCommonBezPath {
        // BezPath can be converted by iterating elements and converting PathEl
        use vello::kurbo::PathEl;
        use vello_common::kurbo::{PathEl as VelloCommonPathEl, Point};

        let mut result = VelloCommonBezPath::new();
        for el in path.elements() {
            // Convert PathEl from vello::kurbo to vello_common::kurbo
            // Points need to be converted as well
            let converted_el = match el {
                PathEl::MoveTo(p) => VelloCommonPathEl::MoveTo(Point::new(p.x, p.y)),
                PathEl::LineTo(p) => VelloCommonPathEl::LineTo(Point::new(p.x, p.y)),
                PathEl::QuadTo(p1, p2) => {
                    VelloCommonPathEl::QuadTo(Point::new(p1.x, p1.y), Point::new(p2.x, p2.y))
                },
                PathEl::CurveTo(p1, p2, p3) => VelloCommonPathEl::CurveTo(
                    Point::new(p1.x, p1.y),
                    Point::new(p2.x, p2.y),
                    Point::new(p3.x, p3.y),
                ),
                PathEl::ClosePath => VelloCommonPathEl::ClosePath,
            };
            result.push(converted_el);
        }
        result
    }

    /// Convert a Brush to vello_common::paint::PaintType.
    ///
    /// This conversion is needed because vello_hybrid uses vello_common types
    /// while the Graphics trait uses vello::peniko::Brush.
    fn brush_to_paint_type(&self, brush: &Brush) -> PaintType {
        match brush {
            Brush::Solid(color) => {
                // Convert peniko::Color to vello_common::peniko::Color
                // Color is AlphaColor<Srgb> and implements Into<PaintType>
                let components = color.components;
                use vello_common::peniko::Color;

                // Convert f32 components [0.0-1.0] to u8 [0-255] for from_rgba8
                let vello_color = Color::from_rgba8(
                    (components[0] * 255.0) as u8,
                    (components[1] * 255.0) as u8,
                    (components[2] * 255.0) as u8,
                    (components[3] * 255.0) as u8,
                );
                vello_color.into()
            },
            Brush::Gradient(gradient) => {
                // Gradient conversion is more complex and may not be fully supported
                // For now, fall back to a solid color
                log::warn!("Gradient brushes in HybridGraphics are not yet fully supported, using solid color fallback");
                let components = gradient
                    .stops
                    .first()
                    .map(|s| s.color.components)
                    .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                use vello_common::peniko::Color;
                let vello_color = Color::from_rgba8(
                    (components[0] * 255.0) as u8,
                    (components[1] * 255.0) as u8,
                    (components[2] * 255.0) as u8,
                    (components[3] * 255.0) as u8,
                );
                vello_color.into()
            },
            Brush::Image(_) => {
                // Image brushes are not yet supported
                log::warn!(
                    "Image brushes in HybridGraphics are not yet supported, using black fallback"
                );
                use vello_common::peniko::color::palette::css::BLACK;
                BLACK.into()
            },
        }
    }

    /// Get the current transform from the stack.
    fn current_transform(&self) -> Affine {
        self.transform_stack
            .last()
            .copied()
            .unwrap_or(Affine::IDENTITY)
    }
}

impl<'a> Graphics for HybridGraphics<'a> {
    fn fill(
        &mut self,
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    ) {
        // Combine the provided transform with the current transform stack
        let combined_transform = self.current_transform() * transform;

        // Set the paint (brush)
        let paint_type = self.brush_to_paint_type(brush);
        self.scene.set_paint(paint_type);

        // Set paint transform if provided
        if let Some(bt) = brush_transform {
            self.scene.set_paint_transform(self.convert_affine(bt));
        } else {
            self.scene.reset_paint_transform();
        }

        // Set fill rule
        self.scene.set_fill_rule(self.convert_fill(fill_rule));

        // Set transform
        self.scene
            .set_transform(self.convert_affine(combined_transform));

        // Fill the path (convert BezPath)
        let converted_path = self.convert_bezpath(shape);
        self.scene.fill_path(&converted_path);
    }

    fn stroke(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    ) {
        // Combine the provided transform with the current transform stack
        let combined_transform = self.current_transform() * transform;

        // Set the paint (brush)
        let paint_type = self.brush_to_paint_type(brush);
        self.scene.set_paint(paint_type);

        // Set paint transform if provided
        if let Some(bt) = brush_transform {
            self.scene.set_paint_transform(self.convert_affine(bt));
        } else {
            self.scene.reset_paint_transform();
        }

        // Set stroke style
        self.scene.set_stroke(self.convert_stroke(style));

        // Set transform
        self.scene
            .set_transform(self.convert_affine(combined_transform));

        // Stroke the path (convert BezPath)
        let converted_path = self.convert_bezpath(shape);
        self.scene.stroke_path(&converted_path);
    }

    fn append(&mut self, _other: &vello::Scene, _transform: Option<Affine>) {
        // Hybrid scenes cannot directly append Vello scenes
        // This is a limitation when mixing backends
        log::warn!(
            "Attempting to append Vello scene to Hybrid scene - this operation is not supported"
        );
        // Note: In practice, widgets should use the unified Scene enum and not mix backends
    }

    fn push_layer(
        &mut self,
        mix: vello::peniko::Mix,
        alpha: f32,
        transform: Affine,
        shape: &BezPath,
    ) {
        // Push transform onto stack
        let new_transform = self.current_transform() * transform;
        self.transform_stack.push(new_transform);

        // Convert Mix to BlendMode
        let blend_mode = BlendMode::new(
            self.convert_mix(mix),
            Compose::SrcOver, // Default compose mode
        );

        // Push layer with clip path (convert BezPath)
        let converted_path = self.convert_bezpath(shape);
        self.scene
            .push_layer(Some(&converted_path), Some(blend_mode), Some(alpha), None);
    }

    fn pop_layer(&mut self) {
        // Pop transform from stack
        if self.transform_stack.len() > 1 {
            self.transform_stack.pop();
        }

        // Pop layer from scene
        self.scene.pop_layer();
    }

    fn as_scene_mut(&mut self) -> Option<&mut vello::Scene> {
        // Hybrid scenes can't be converted to Vello scenes
        // This is used for Parley text rendering, which requires vello::Scene
        // For Hybrid backend, text rendering may need a different approach
        None
    }
}
