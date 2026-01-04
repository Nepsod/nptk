//! File icon widget for rendering file icons with async loading and caching.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::{Update, UpdateManager};
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
use nptk_core::vg::peniko::{Blob, Brush, Color, Fill, ImageAlphaType, ImageBrush, ImageData, ImageFormat};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use npio::service::icon::{CachedIcon, IconRegistry};
use npio::get_file_for_uri;
use nptk_services::filesystem::entry::{FileEntry, FileType};
use nptk_services::thumbnail::npio_adapter::file_entry_to_uri;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use vello_svg;

/// Widget for rendering file icons with async loading and caching.
///
/// This widget loads icons asynchronously, caches them per (path, size) key,
/// and caches parsed SVG scenes to avoid re-parsing.
pub struct FileIcon {
    /// File entry.
    entry: MaybeSignal<FileEntry>,
    /// Icon size.
    size: MaybeSignal<u32>,
    /// Layout style.
    layout_style: MaybeSignal<LayoutStyle>,
    /// Icon registry for loading icons.
    icon_registry: Arc<IconRegistry>,
    /// Icon cache: (path, size) -> CachedIcon.
    icon_cache: Arc<Mutex<HashMap<(PathBuf, u32), Option<CachedIcon>>>>,
    /// SVG scene cache: SVG source string -> (Scene, width, height).
    svg_scene_cache: HashMap<String, (nptk_core::vg::Scene, f64, f64)>,
    /// Cache update sender for triggering redraws.
    cache_update_tx: tokio::sync::mpsc::UnboundedSender<()>,
    /// Cache update receiver for checking if cache was updated.
    cache_update_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<()>>>,
    /// Update manager for triggering redraws when cache updates.
    update_manager: Arc<Mutex<Option<UpdateManager>>>,
}

impl FileIcon {
    /// Create a new file icon widget.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to load icon for
    /// * `size` - The icon size in pixels
    pub fn new(entry: FileEntry, size: u32) -> Self {
        let icon_registry = Arc::new(
            IconRegistry::new().unwrap_or_else(|_| IconRegistry::default())
        );
        let icon_cache = Arc::new(Mutex::new(HashMap::new()));
        let (cache_update_tx, cache_update_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            entry: entry.into(),
            size: size.into(),
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::length(size as f32), Dimension::length(size as f32)),
                ..Default::default()
            }
            .into(),
            icon_registry,
            icon_cache,
            svg_scene_cache: HashMap::new(),
            cache_update_tx,
            cache_update_rx: Arc::new(Mutex::new(cache_update_rx)),
            update_manager: Arc::new(Mutex::new(None)),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Set the file entry.
    pub fn with_entry(self, entry: impl Into<MaybeSignal<FileEntry>>) -> Self {
        self.apply_with(|s| s.entry = entry.into())
    }

    /// Set the icon size.
    pub fn with_size(self, size: u32) -> Self {
        self.apply_with(|s| s.set_size(size))
    }

    fn set_size(&mut self, size: u32) {
        self.size = size.into();
        self.layout_style = LayoutStyle {
            size: Vector2::new(Dimension::length(size as f32), Dimension::length(size as f32)),
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
        use nptk_core::vg::peniko::{
            Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
        };
        let image_data = ImageData {
            data: Blob::from(data.to_vec()),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width,
            height,
        };
        let image_brush = ImageBrush::new(image_data);
        let scale_x = size / (width as f64);
        let scale_y = size / (height as f64);
        let scale = scale_x.min(scale_y);
        let transform = Affine::scale_non_uniform(scale, scale)
            .then_translate(Vec2::new(x, y));
        if let Some(scene) = graphics.as_scene_mut() {
            scene.draw_image(&image_brush, transform);
        }
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

    fn update(&mut self, _: &LayoutNode, context: AppContext, _: &mut AppInfo) -> Update {
        // Store UpdateManager for triggering redraws when cache updates
        {
            let mut update_manager = self.update_manager.lock().unwrap();
            *update_manager = Some(context.update());
        }

        let mut update = Update::empty();

        // Poll cache update notifications (non-blocking)
        if let Ok(mut rx) = self.cache_update_rx.try_lock() {
            while rx.try_recv().is_ok() {
                update.insert(Update::DRAW);
            }
        }

        // Poll cache update notifications and trigger redraws
        // Icon loading happens in render() method, not here

        update
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let entry = self.entry.get().clone();
        let path = entry.path.clone();
        let size = *self.size.get();
        let (x, y, render_size) = Self::layout_origin_size(layout);
        let icon_rect = Rect::new(x, y, x + render_size, y + render_size);

        let cache_key = (path.clone(), size);
        let cached_icon = {
            let cache = self.icon_cache.lock().unwrap();
            cache.get(&cache_key).and_then(|opt| opt.clone())
        };

        // If icon not cached, request it asynchronously (non-blocking)
        if cached_icon.is_none() {
            let cache_clone = self.icon_cache.clone();
            let registry_clone = self.icon_registry.clone();
            let entry_clone = entry.clone();
            let cache_key_clone = cache_key.clone();
            let cache_update_tx_clone = self.cache_update_tx.clone();
            tokio::spawn(async move {
                let uri = file_entry_to_uri(&entry_clone);
                if let Ok(file) = get_file_for_uri(&uri) {
                    let icon = registry_clone.get_file_icon(&*file, size).await;
                    let mut cache = cache_clone.lock().unwrap();
                    cache.insert(cache_key_clone, icon);
                    // Notify that cache was updated to trigger redraw
                    let _ = cache_update_tx_clone.send(());
                }
            });
        }

        if let Some(icon) = cached_icon {
            let icon_x = icon_rect.x0;
            let icon_y = icon_rect.y0;
            let icon_size_f64 = icon_rect.width().min(icon_rect.height());

            match icon {
                npio::service::icon::CachedIcon::Image {
                    data,
                    width,
                    height,
                } => {
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
                    let scale_x = icon_size_f64 / (width as f64);
                    let scale_y = icon_size_f64 / (height as f64);
                    let scale = scale_x.min(scale_y);
                    let transform = Affine::scale_non_uniform(scale, scale)
                        .then_translate(Vec2::new(icon_x, icon_y));
                    if let Some(scene) = graphics.as_scene_mut() {
                        scene.draw_image(&image_brush, transform);
                    }
                },
                npio::service::icon::CachedIcon::Svg(svg_source) => {
                    // Check SVG scene cache first
                    let cached_scene =
                        self.svg_scene_cache.get(svg_source.as_str()).cloned();
                    let (scene, svg_width, svg_height) = if let Some((scene, w, h)) =
                        cached_scene
                    {
                        (scene, w, h)
                    } else {
                        // Cache miss - parse and render SVG
                        use vello_svg::usvg::{
                            ImageRendering, Options, ShapeRendering, TextRendering, Tree,
                        };
                        if let Ok(tree) = Tree::from_str(
                            svg_source.as_str(),
                            &Options {
                                shape_rendering: ShapeRendering::GeometricPrecision,
                                text_rendering: TextRendering::OptimizeLegibility,
                                image_rendering: ImageRendering::OptimizeSpeed,
                                ..Default::default()
                            },
                        ) {
                            let scene = vello_svg::render_tree(&tree);
                            let svg_size = tree.size();
                            let w = svg_size.width() as f64;
                            let h = svg_size.height() as f64;
                            self.svg_scene_cache.insert(
                                svg_source.as_str().to_string(),
                                (scene.clone(), w, h),
                            );
                            (scene, w, h)
                        } else {
                            (nptk_core::vg::Scene::new(), 1.0, 1.0)
                        }
                    };

                    let scale_x = icon_size_f64 / svg_width;
                    let scale_y = icon_size_f64 / svg_height;
                    let scale = scale_x.min(scale_y);
                    let transform = Affine::scale_non_uniform(scale, scale)
                        .then_translate(Vec2::new(icon_x, icon_y));
                    graphics.append(&scene, Some(transform));
                },
                    npio::service::icon::CachedIcon::Path(_) => {
                    let icon_color = theme
                        .get_property(
                            self.widget_id(),
                            &nptk_theme::properties::ThemeProperty::ColorText,
                        )
                        .or_else(|| {
                            theme.get_default_property(
                                &nptk_theme::properties::ThemeProperty::ColorText,
                            )
                        })
                        .unwrap_or(Color::from_rgb8(150, 150, 150));

                    // Determine if it's a directory using FileEntry
                    let is_directory = entry.file_type == FileType::Directory;
                    let fallback_color = if is_directory {
                        icon_color.with_alpha(0.6)
                    } else {
                        icon_color.with_alpha(0.4)
                    };

                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(fallback_color),
                        None,
                        &icon_rect.to_path(0.1),
                    );
                },
            }
        } else {
            let icon_color = theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorText,
                )
                .or_else(|| {
                    theme.get_default_property(
                        &nptk_theme::properties::ThemeProperty::ColorText,
                    )
                })
                .unwrap_or(Color::from_rgb8(150, 150, 150));

            // Try to determine if it's a directory using FileEntry
            let is_directory = entry.file_type == FileType::Directory;
            let fallback_color = if is_directory {
                icon_color.with_alpha(0.6)
            } else {
                icon_color.with_alpha(0.4)
            };

            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(fallback_color),
                None,
                &icon_rect.to_path(0.1),
            );
        }
    }
}

impl WidgetLayoutExt for FileIcon {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
