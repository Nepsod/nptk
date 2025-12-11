use super::FileListContent;
use nptk_core::app::info::AppInfo;
use nptk_core::layout::LayoutNode;
use nptk_core::signal::Signal;
use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::Widget;
use nptk_services::filesystem::entry::FileType;
use nptk_theme::theme::Theme;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

impl FileListContent {
    pub(super) fn render_list_view(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
    ) {
        use std::time::Instant;
        let start = Instant::now();

        let entries = self.entries.get();
        let selected_paths = self.selected_paths.get();
        let selected_set: HashSet<&PathBuf> = selected_paths.iter().collect();
        let entry_count = entries.len();

        // Draw background
        let bg_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        let bg_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorBackground,
            )
            .or_else(|| {
                theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackground)
            })
            .unwrap_or_else(|| theme.window_background());

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &bg_rect.to_path(0.1),
        );

        if entry_count == 0 {
            return;
        }

        // VIEWPORT CULLING: Calculate visible range
        // VIEWPORT CULLING: Calculate visible range relative to window
        let viewport_start_y = (-layout.layout.location.y).max(0.0);
        let viewport_end_y = info.size.y as f32 - layout.layout.location.y;

        let start_index = (viewport_start_y / self.item_height).floor().max(0.0) as usize;
        let end_index = ((viewport_end_y / self.item_height).ceil() as usize + 1).min(entry_count);

        // Only render visible items
        for i in start_index..end_index {
            let layout_start = Instant::now();
            let entry = &entries[i];
            let y = layout.layout.location.y + i as f32 * self.item_height;
            let row_rect = Rect::new(
                layout.layout.location.x as f64,
                y as f64,
                (layout.layout.location.x + layout.layout.size.width) as f64,
                (y + self.item_height) as f64,
            );

            // Check for hover state
            let is_hovered = if let Some(cursor) = info.cursor_pos {
                let cursor_x = cursor.x as f64;
                let cursor_y = cursor.y as f64;
                cursor_x >= row_rect.x0
                    && cursor_x < row_rect.x1
                    && cursor_y >= row_rect.y0
                    && cursor_y < row_rect.y1
            } else {
                false
            };

            // Draw hover background (if not selected)
            if is_hovered && !selected_set.contains(&entry.path) {
                let hover_color = theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::ColorMenuHovered,
                    )
                    .or_else(|| {
                        theme.get_default_property(
                            &nptk_theme::properties::ThemeProperty::ColorMenuHovered,
                        )
                    })
                    .unwrap_or_else(|| Color::from_rgb8(240, 240, 240));

                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(hover_color),
                    None,
                    &row_rect.to_path(0.1),
                );
            }

            // Draw selection background
            if selected_set.contains(&entry.path) {
                let color = theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected,
                    )
                    .or_else(|| {
                        theme.get_default_property(
                            &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected,
                        )
                    })
                    .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));

                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(0.3)),
                    None,
                    &row_rect.to_path(0.1),
                );
            }

            // Try to get thumbnail first, fall back to icon
            let icon_size = 20.0;
            let icon_rect = Rect::new(
                row_rect.x0 + 5.0,
                row_rect.y0 + 5.0,
                row_rect.x0 + 25.0,
                row_rect.y1 - 5.0,
            );

            // Check if thumbnail is available
            let mut use_thumbnail = false;
            if let Some(thumbnail_path) = self
                .thumbnail_provider
                .get_thumbnail(entry, self.thumbnail_size)
            {
                if let Ok(Some(cached_thumb)) = self
                    .thumbnail_cache
                    .load_or_get(&thumbnail_path, self.thumbnail_size)
                {
                    use nptk_core::vg::peniko::{
                        Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
                    };
                    let image_data = ImageData {
                        data: Blob::from(cached_thumb.data.as_ref().clone()),
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width: cached_thumb.width,
                        height: cached_thumb.height,
                    };
                    let image_brush = ImageBrush::new(image_data);
                    let icon_x = icon_rect.x0;
                    let icon_y = icon_rect.y0;
                    let icon_size_f64 = icon_rect.width().min(icon_rect.height());
                    let scale_x = icon_size_f64 / (cached_thumb.width as f64);
                    let scale_y = icon_size_f64 / (cached_thumb.height as f64);
                    let scale = scale_x.min(scale_y);
                    let transform = Affine::scale_non_uniform(scale, scale)
                        .then_translate(Vec2::new(icon_x, icon_y));
                    if let Some(scene) = graphics.as_scene_mut() {
                        scene.draw_image(&image_brush, transform);
                    }
                    use_thumbnail = true;
                }
            }

            // If no thumbnail, use icon
            if !use_thumbnail {
                if self.thumbnail_provider.is_supported(entry) {
                    let mut pending = self.pending_thumbnails.lock().unwrap();
                    if !pending.contains(&entry.path) {
                        if let Ok(()) = self
                            .thumbnail_provider
                            .request_thumbnail(entry, self.thumbnail_size)
                        {
                            pending.insert(entry.path.clone());
                        }
                    }
                }

                let cache_key = (entry.path.clone(), icon_size as u32);
                let cached_icon = {
                    let mut cache = self.icon_cache.lock().unwrap();
                    if let Some(icon) = cache.get(&cache_key) {
                        icon.clone()
                    } else {
                        let icon = self.icon_registry.get_file_icon(entry, icon_size as u32);
                        cache.insert(cache_key.clone(), icon.clone());
                        icon
                    }
                };

                if let Some(icon) = cached_icon {
                    let icon_x = icon_rect.x0;
                    let icon_y = icon_rect.y0;
                    let icon_size_f64 = icon_rect.width().min(icon_rect.height());

                    match icon {
                        nptk_services::icon::CachedIcon::Image {
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
                        nptk_services::icon::CachedIcon::Svg(svg_source) => {
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
                        nptk_services::icon::CachedIcon::Path(_) => {
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

                            let fallback_color = if entry.file_type == FileType::Directory {
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

                    let fallback_color = if entry.file_type == FileType::Directory {
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

            // Draw text
            let text_color = theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorText,
                )
                .or_else(|| {
                    theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText)
                })
                .unwrap_or(Color::BLACK);

            let transform = Affine::translate((row_rect.x0 + 35.0, row_rect.y0 + 5.0));

            self.text_render_context.render_text(
                &mut info.font_context,
                graphics,
                &entry.name,
                None,
                16.0,
                Brush::Solid(text_color),
                transform,
                true,
                Some(row_rect.width() as f32 - 40.0),
            );
        }

        let total_duration = start.elapsed();

        // DEBUG: Log timing every 60 frames
        // use std::sync::atomic::{AtomicU64, Ordering};
        // static FRAME_COUNT: AtomicU64 = AtomicU64::new(0);
        // let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        // if frame % 60 == 0 {
        // }
    }
}
