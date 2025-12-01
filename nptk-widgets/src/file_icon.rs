//! File icon widget for rendering file icons in file lists.

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Vec2};
use nptk_core::vg::peniko::{Blob, ImageBrush, ImageData, ImageFormat, ImageAlphaType};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_services::icon::CachedIcon;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use vello_svg::usvg::Options;
use vello_svg::usvg::{ShapeRendering, TextRendering, ImageRendering};

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

    /// Set the icon.
    pub fn with_icon(mut self, icon: impl Into<MaybeSignal<Option<CachedIcon>>>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the icon size.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self.layout_style = LayoutStyle {
            size: Vector2::new(Dimension::length(size), Dimension::length(size)),
            ..Default::default()
        }
        .into();
        self
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
        let icon = match *self.icon.get() {
            Some(ref icon) => icon.clone(),
            None => return,
        };

        let x = layout.layout.location.x as f64;
        let y = layout.layout.location.y as f64;
        let size = layout.layout.size.width.min(layout.layout.size.height) as f64;

        match icon {
            CachedIcon::Image { data, width, height } => {
                // Create ImageData from raw RGBA bytes
                let image_data = ImageData {
                    data: Blob::from(data.as_ref().clone()),
                    format: ImageFormat::Rgba8,
                    alpha_type: ImageAlphaType::Alpha,
                    width,
                    height,
                };

                let image_brush = ImageBrush::new(image_data);

                // Scale to fit the layout size
                let scale_x = size / (width as f64);
                let scale_y = size / (height as f64);
                let scale = scale_x.min(scale_y);

                let transform = Affine::scale_non_uniform(scale, scale)
                    .then_translate(Vec2::new(x, y));

                if let Some(scene) = graphics.as_scene_mut() {
                    scene.draw_image(&image_brush, transform);
                }
            }
            CachedIcon::Svg(svg_source) => {
                // Parse and render SVG
                let tree = match vello_svg::usvg::Tree::from_str(
                    svg_source.as_str(),
                    &Options {
                        shape_rendering: ShapeRendering::GeometricPrecision,
                        text_rendering: TextRendering::OptimizeLegibility,
                        image_rendering: ImageRendering::OptimizeSpeed,
                        ..Default::default()
                    },
                ) {
                    Ok(tree) => tree,
                    Err(_) => return,
                };

                let scene = vello_svg::render_tree(&tree);

                // Scale to fit the layout size
                let svg_size = tree.size();
                let scale_x = size / svg_size.width() as f64;
                let scale_y = size / svg_size.height() as f64;
                let scale = scale_x.min(scale_y);

                let transform = Affine::scale_non_uniform(scale, scale)
                    .then_translate(Vec2::new(x, y));

                graphics.append(&scene, Some(transform));
            }
            CachedIcon::Path(_) => {
                // Path-based icons should be loaded before rendering
                // This case shouldn't happen if the registry is used correctly
            }
        }
    }
}

impl WidgetLayoutExt for FileIcon {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

