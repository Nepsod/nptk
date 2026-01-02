//! File icon widget for rendering file icons in file lists.

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Vec2};
use nptk_core::vg::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use npio::service::icon::CachedIcon;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use vello_svg::usvg::Options;
use vello_svg::usvg::{ImageRendering, ShapeRendering, TextRendering};

/// Widget for rendering file icons (PNG or SVG).
pub struct FileIcon {
    /// Cached icon data.
    icon: MaybeSignal<Option<CachedIcon>>,
    /// Icon size.
    size: f32,
    /// Layout style.
    layout_style: MaybeSignal<LayoutStyle>,
}

impl FileIcon {
    /// Create a new file icon widget.
    pub fn new(icon: Option<CachedIcon>, size: f32) -> Self {
        Self {
            icon: icon.into(),
            size,
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::length(size), Dimension::length(size)),
                ..Default::default()
            }
            .into(),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Set the icon.
    pub fn with_icon(self, icon: impl Into<MaybeSignal<Option<CachedIcon>>>) -> Self {
        self.apply_with(|s| s.icon = icon.into())
    }

    /// Set the icon size.
    pub fn with_size(self, size: f32) -> Self {
        self.apply_with(|s| s.set_size(size))
    }

    fn set_size(&mut self, size: f32) {
        self.size = size;
        self.layout_style = LayoutStyle {
            size: Vector2::new(Dimension::length(size), Dimension::length(size)),
            ..Default::default()
        }
        .into();
    }

    fn layout_origin_size(layout: &LayoutNode) -> (f64, f64, f64) {
        let x = layout.layout.location.x as f64;
        let y = layout.layout.location.y as f64;
        let size = layout.layout.size.width.min(layout.layout.size.height) as f64;
        (x, y, size)
    }

    fn render_image(
        graphics: &mut dyn Graphics,
        data: &[u8],
        width: u32,
        height: u32,
        x: f64,
        y: f64,
        size: f64,
    ) {
        let image_data = ImageData {
            data: Blob::from(data.to_vec()),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width,
            height,
        };

        let image_brush = ImageBrush::new(image_data);

        let scale_x = size / width as f64;
        let scale_y = size / height as f64;
        let scale = scale_x.min(scale_y);

        let transform = Affine::scale_non_uniform(scale, scale).then_translate(Vec2::new(x, y));

        if let Some(scene) = graphics.as_scene_mut() {
            scene.draw_image(&image_brush, transform);
        }
    }

    fn render_svg(
        graphics: &mut dyn Graphics,
        svg_source: &str,
        x: f64,
        y: f64,
        size: f64,
    ) {
        let options = Options {
            shape_rendering: ShapeRendering::GeometricPrecision,
            text_rendering: TextRendering::OptimizeLegibility,
            image_rendering: ImageRendering::OptimizeSpeed,
            ..Default::default()
        };

        let tree = match vello_svg::usvg::Tree::from_str(svg_source, &options) {
            Ok(tree) => tree,
            Err(err) => {
                log::warn!("FileIcon: failed to parse SVG: {err}");
                return;
            },
        };

        let scene = vello_svg::render_tree(&tree);
        let svg_size = tree.size();
        let scale_x = size / svg_size.width() as f64;
        let scale_y = size / svg_size.height() as f64;
        let scale = scale_x.min(scale_y);

        let transform = Affine::scale_non_uniform(scale, scale).then_translate(Vec2::new(x, y));
        graphics.append(&scene, Some(transform));
    }
}

impl Widget for FileIcon {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "FileIcon")
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![],
        }
    }

    fn update(&mut self, _: &LayoutNode, _: AppContext, _: &mut AppInfo) -> Update {
        Update::empty()
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        _: &mut dyn Theme,
        layout: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let icon_value = self.icon.get();
        let Some(icon) = icon_value.as_ref() else { return };

        let (x, y, size) = Self::layout_origin_size(layout);

        match icon {
            CachedIcon::Image {
                data,
                width,
                height,
            } => {
                Self::render_image(graphics, data.as_ref(), *width, *height, x, y, size);
            },
            CachedIcon::Svg(svg_source) => {
                Self::render_svg(graphics, svg_source, x, y, size);
            },
            CachedIcon::Path(_) => {
                log::warn!("FileIcon: CachedIcon::Path encountered at render time; expected preloaded image/svg");
            },
        }
    }
}

impl WidgetLayoutExt for FileIcon {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
