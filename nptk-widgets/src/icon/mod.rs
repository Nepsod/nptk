use nalgebra::Vector2;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::vg::kurbo::{Affine, Vec2};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::sync::Arc;
use vello_svg::usvg;

use crate::icon::svg::SvgIcon;
use nptk_core::app::context::AppContext;
use nptk_core::signal::MaybeSignal;
pub use usvg::ImageRendering;
pub use usvg::ShapeRendering;
pub use usvg::TextRendering;

/// Contains the [SvgIcon] struct for representing a rendered SVG Icon.
pub mod svg;

/// Error type for parsing SVGs with [usvg].
pub type SvgError = usvg::Error;

/// Icon data that can be either SVG or Image format.
#[derive(Clone)]
enum IconData {
    /// SVG icon (rendered as Scene).
    Svg(SvgIcon),
    /// Image icon (PNG/XPM - RGBA pixel data).
    Image {
        data: Arc<Vec<u8>>,
        width: u32,
        height: u32,
    },
}

/// A simple icon widget to display icons from XDG icon theme or SVG sources.
///
/// Supports both SVG icons (from XDG theme or direct SVG source) and Image icons (PNG/XPM from XDG theme).
///
/// ### Theming
/// The widget itself only draws the underlying icon, so theming is useless.
pub struct Icon {
    layout_style: MaybeSignal<LayoutStyle>,
    icon: MaybeSignal<IconData>,
}

impl Icon {
    /// Creates a new icon widget from an XDG icon name (e.g., "user-home", "folder", "application-pdf").
    ///
    /// The icon will be loaded from the system's XDG icon theme.
    ///
    /// # Arguments
    /// * `icon_name` - The XDG icon name (e.g., "user-home", "folder-open")
    /// * `size` - The desired icon size in pixels
    /// * `registry` - Optional shared icon registry. If None, a new registry will be created.
    pub fn new(
        icon_name: impl Into<String>,
        size: u32,
        registry: Option<Arc<nptk_services::icon::IconRegistry>>,
    ) -> Self {
        use nptk_services::icon::{CachedIcon, IconRegistry};

        let icon_name = icon_name.into();
        let registry = registry.unwrap_or_else(|| {
            Arc::new(
                IconRegistry::new().unwrap_or_else(|_| IconRegistry::default()),
            )
        });

        // Load icon from XDG theme
        let cached_icon = registry.get_icon(&icon_name, size);

        // Convert CachedIcon to IconData
        let icon_data = match cached_icon {
            Some(CachedIcon::Svg(svg_source)) => {
                // Parse SVG string to SvgIcon
                match SvgIcon::new(svg_source.as_str()) {
                    Ok(svg_icon) => IconData::Svg(svg_icon),
                    Err(e) => {
                        log::warn!(
                            "Failed to parse SVG for icon '{}': {}",
                            icon_name,
                            e
                        );
                        IconData::Svg(SvgIcon::from(nptk_core::vg::Scene::new()))
                    },
                }
            },
            Some(CachedIcon::Image { data, width, height }) => {
                IconData::Image { data, width, height }
            },
            Some(CachedIcon::Path(_)) => {
                // Path variant is for lazy loading, not supported here
                log::warn!(
                    "Icon '{}' returned Path variant (lazy loading not supported)",
                    icon_name
                );
                IconData::Svg(SvgIcon::from(nptk_core::vg::Scene::new()))
            },
            None => {
                log::warn!("Icon '{}' not found in XDG theme", icon_name);
                IconData::Svg(SvgIcon::from(nptk_core::vg::Scene::new()))
            },
        };

        Self {
            layout_style: LayoutStyle {
                size: Vector2::new(
                    Dimension::length(size as f32),
                    Dimension::length(size as f32),
                ),
                ..Default::default()
            }
            .into(),
            icon: MaybeSignal::value(icon_data),
        }
    }

    /// Creates a new icon widget from an SVG source string or file path.
    ///
    /// This is the original constructor, renamed for clarity.
    pub fn from_svg(icon: impl Into<MaybeSignal<SvgIcon>>) -> Self {
        Self {
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::length(8.0), Dimension::length(8.0)),
                ..Default::default()
            }
            .into(),
            icon: {
                let icon_signal = icon.into();
                // Convert MaybeSignal<SvgIcon> to MaybeSignal<IconData>
                // We need to map the signal to wrap SvgIcon in IconData
                icon_signal.map(|svg_icon_ref| {
                    use nptk_core::reference::Ref;
                    use std::rc::Rc;
                    // Dereference Ref to get &SvgIcon, then clone and wrap in IconData
                    let svg_icon = (*svg_icon_ref).clone();
                    Ref::Rc(Rc::new(IconData::Svg(svg_icon)))
                })
            },
        }
    }
}

impl Widget for Icon {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        _: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let icon_data_ref = self.icon.get();

        match *icon_data_ref {
            IconData::Svg(ref svg_icon) => {
                // The size is divided, as otherwise the icon would be either too large (with 1.0) or too tiny (with 0.1 somehow getting converted to 0.0)
                let affine = Affine::scale_non_uniform(
                    layout_node.layout.size.width as f64 / 100.0,
                    layout_node.layout.size.height as f64 / 100.0,
                )
                .then_translate(Vec2::new(
                    layout_node.layout.location.x as f64,
                    layout_node.layout.location.y as f64,
                ));

                graphics.append(&svg_icon.scene(), Some(affine));
            },
            IconData::Image { ref data, width, height } => {
                use nptk_core::vg::peniko::{
                    Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
                };

                let image_data = ImageData {
                    data: Blob::from(data.as_ref().clone()),
                    format: ImageFormat::Rgba8,
                    alpha_type: ImageAlphaType::Alpha,
                    width,
                    height,
                };
                let image_brush = ImageBrush::new(image_data);

                // Scale image to fit layout size while maintaining aspect ratio
                let scale_x = layout_node.layout.size.width as f64 / (width as f64);
                let scale_y = layout_node.layout.size.height as f64 / (height as f64);
                let scale = scale_x.min(scale_y);

                let transform = Affine::scale_non_uniform(scale, scale)
                    .then_translate(Vec2::new(
                        layout_node.layout.location.x as f64,
                        layout_node.layout.location.y as f64,
                    ));

                if let Some(scene) = graphics.as_scene_mut() {
                    scene.draw_image(&image_brush, transform);
                }
            },
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, _: &LayoutNode, _: AppContext, _: &mut AppInfo) -> Update {
        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Icon")
    }
}

impl WidgetLayoutExt for Icon {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
