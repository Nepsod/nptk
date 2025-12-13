use crate::icon::{ImageRendering, ShapeRendering, SvgError, TextRendering};
use nptk_core::vg::Scene;
use std::sync::Arc;
use vello_svg::usvg;
use vello_svg::usvg::Options;

/// An SVG icon rendered as a Vello [Scene].
#[derive(Clone)]
pub struct SvgIcon {
    scene: Arc<Scene>,
    width: f64,
    height: f64,
}

impl SvgIcon {
    /// Creates a new icon from the given SVG source.
    /// Returns [Ok] if the SVG could be parsed, [Err] otherwise.
    ///
    /// **This calls [Self::new_custom] with the following options:**
    /// - [ShapeRendering::GeometricPrecision] for precise shape rendering.
    /// - [TextRendering::OptimizeLegibility] for good text rendering.
    /// - [ImageRendering::OptimizeSpeed] for fast image rendering.
    pub fn new(source: impl AsRef<str>) -> Result<Self, SvgError> {
        Self::new_custom(
            source,
            ShapeRendering::GeometricPrecision,
            TextRendering::OptimizeLegibility,
            ImageRendering::OptimizeSpeed,
        )
    }

    /// Creates a new icon from the given SVG source.
    /// Returns [Ok] if the SVG could be parsed, [Err] otherwise.
    ///
    /// This method allows customizing the SVG rendering options.
    pub fn new_custom(
        source: impl AsRef<str>,
        shape_rendering: ShapeRendering,
        text_rendering: TextRendering,
        image_rendering: ImageRendering,
    ) -> Result<Self, SvgError> {
        let tree = usvg::Tree::from_str(
            source.as_ref(),
            &Options {
                shape_rendering,
                text_rendering,
                image_rendering,
                ..Default::default()
            },
        )?;

        let scene = vello_svg::render_tree(&tree);
        let svg_size = tree.size();
        let width = svg_size.width() as f64;
        let height = svg_size.height() as f64;

        Ok(Self {
            scene: Arc::new(scene),
            width,
            height,
        })
    }

    /// Returns the underlying [Scene].
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Returns the SVG's natural width.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns the SVG's natural height.
    pub fn height(&self) -> f64 {
        self.height
    }
}

impl From<Scene> for SvgIcon {
    fn from(scene: Scene) -> Self {
        // Default to 100x100 for backwards compatibility when creating from Scene directly
        Self {
            scene: Arc::new(scene),
            width: 100.0,
            height: 100.0,
        }
    }
}
