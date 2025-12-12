use super::{FileListContent, FileListViewMode};
use nptk_core::app::font_ctx::FontContext;
use nptk_core::app::info::AppInfo;
use nptk_core::layout::LayoutNode;
use nptk_core::signal::Signal;
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::Widget;
use nptk_services::filesystem::entry::{FileEntry, FileType};
use nptk_theme::theme::Theme;
use std::collections::HashSet;
use std::path::PathBuf;

impl FileListContent {
    pub(super) fn calculate_compact_view_layout(&self, width: f32) -> (usize, f32, f32, f32) {
        let cell_width = 250.0; // Fixed width for compact tiles
        let cell_height = 60.0; // Fixed height for compact tiles
        let spacing = 10.0; // Spacing between tiles

        let available_width = width - self.icon_view_padding * 2.0;
        let columns = ((available_width + spacing) / (cell_width + spacing)).floor() as usize;
        let columns = columns.max(1);

        (columns, cell_width, cell_height, spacing)
    }
    pub(super) fn get_compact_item_layout(
        &mut self,
        font_cx: &mut FontContext,
        entry: &FileEntry,
        cell_height: f32,
        cell_width: f32,
    ) -> (Rect, Rect) {
        // Check cache first
        let cache_key = (
            entry.path.clone(),
            FileListViewMode::Compact,
            cell_width as u32,
            false,
        );
        if let Some((icon_rect, label_rect, _, _)) = self.layout_cache.get(&cache_key) {
            return (*icon_rect, *label_rect);
        }

        // Define Icon area (relative to 0,0)
        let icon_size = 32.0f32;
        let icon_padding = 8.0f32;
        let icon_x = icon_padding as f64;
        let icon_y = (cell_height as f64 - icon_size as f64) / 2.0;
        let icon_rect = Rect::new(
            icon_x,
            icon_y,
            icon_x + icon_size as f64,
            icon_y + icon_size as f64,
        );

        let text_x = icon_x + icon_size as f64 + 10.0;
        let text_y = 12.0; // Relative to top of cell
        let max_text_width = (cell_width - (icon_padding + icon_size + 10.0 + 8.0)) as usize;
        let font_size = 14.0;

        // Measure text to determine label width
        let (text_width, line_count) = self.text_render_context.measure_text_layout(
            font_cx,
            &entry.name,
            font_size,
            Some(max_text_width as f32),
        );

        let display_lines = line_count.min(2);
        let text_height = display_lines as f32 * (font_size * 1.2); // Approx height

        // Label rect (tight fit around text)
        let label_padding_x = 4.0;
        let label_padding_y = 2.0;
        let label_rect = Rect::new(
            text_x - label_padding_x,
            text_y - label_padding_y,
            text_x + text_width as f64 + label_padding_x,
            text_y + text_height as f64 + label_padding_y,
        );

        // Cache the result (with empty string and 0.0 for unused fields)
        let result = (icon_rect, label_rect);
        self.layout_cache
            .insert(cache_key, (icon_rect, label_rect, String::new(), 0.0));

        result
    }
    pub(super) fn render_compact_view(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
    ) {
        let entries = self.entries.get();
        let selected_paths = self.selected_paths.get();
        let selected_set: HashSet<PathBuf> = selected_paths.iter().cloned().collect();

        let (columns, cell_width, cell_height, spacing) =
            self.calculate_compact_view_layout(layout.layout.size.width);

        // VIEWPORT CULLING: Calculate visible range relative to window
        // layout.layout.location.y includes the scroll offset (negative when scrolled down)
        // and the widget's position in the window.
        let viewport_start_y = (-layout.layout.location.y).max(0.0);
        let viewport_end_y = info.size.y as f32 - layout.layout.location.y;

        let row_height = cell_height + spacing;
        let start_row = (viewport_start_y / row_height).floor().max(0.0) as usize;
        let end_row = (viewport_end_y / row_height).ceil() as usize + 1;

        let start_index = start_row * columns;
        let end_index = (end_row * columns).min(entries.len());

        // Collect visible entries to avoid borrow checker issues
        let visible_entries: Vec<(usize, FileEntry)> = (start_index..end_index)
            .map(|i| (i, entries[i].clone()))
            .collect();

        // Drop the signal references to release the borrow
        drop(entries);
        drop(selected_paths);

        // Only render visible items
        for (i, entry) in &visible_entries {
            let row = i / columns;
            let col = i % columns;
            let x = layout.layout.location.x
                + self.icon_view_padding
                + col as f32 * (cell_width + spacing);
            let y = layout.layout.location.y
                + self.icon_view_padding
                + row as f32 * (cell_height + spacing);

            let is_selected = selected_set.contains(&entry.path);

            // Calculate layout
            let (mut icon_rect, mut label_rect) = self.get_compact_item_layout(
                &mut info.font_context,
                &entry,
                cell_height,
                cell_width,
            );

            // Translate relative layout to absolute position
            icon_rect = icon_rect + Vec2::new(x as f64, y as f64);
            label_rect = label_rect + Vec2::new(x as f64, y as f64);

            // Check for hover state
            let is_hovered = if let Some(cursor) = info.cursor_pos {
                let cursor_x = cursor.x as f64;
                let cursor_y = cursor.y as f64;
                // Check against the full cell rect for hover
                cursor_x >= x as f64
                    && cursor_x < (x + cell_width) as f64
                    && cursor_y >= y as f64
                    && cursor_y < (y + cell_height) as f64
            } else {
                false
            };

            // Extract layout properties for rendering
            let icon_x = icon_rect.x0;
            let icon_y = icon_rect.y0;
            let icon_size = icon_rect.width() as f32;
            let font_size = 14.0;

            // 1. Draw Label Background (Selection/Hover)
            if is_selected || is_hovered {
                let color_prop = if is_selected {
                    nptk_theme::properties::ThemeProperty::ColorBackgroundSelected
                } else {
                    nptk_theme::properties::ThemeProperty::ColorMenuHovered
                };

                let color = theme
                    .get_property(self.widget_id(), &color_prop)
                    .or_else(|| theme.get_default_property(&color_prop))
                    .unwrap_or_else(|| {
                        if is_selected {
                            Color::from_rgb8(100, 150, 255)
                        } else {
                            Color::from_rgb8(240, 240, 240)
                        }
                    });

                let alpha = if is_selected { 0.7 } else { 0.5 };

                let label_bg_rect =
                    RoundedRect::from_rect(label_rect, RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0));

                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(alpha)),
                    None,
                    &label_bg_rect.to_path(0.1),
                );
            }

            // 2. Draw Icon
            // Try to get thumbnail first, fall back to icon
            let mut use_thumbnail = false;
            let thumb_size = 48;

            if let Some(thumbnail_path) = self.thumbnail_provider.get_thumbnail(&entry, thumb_size)
            {
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    if let Ok(Some(cached_thumb)) = handle.block_on(async {
                        self.thumbnail_cache
                            .load_or_get(&thumbnail_path, thumb_size)
                            .await
                    }) {
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
                    let scale_x = icon_size as f64 / (cached_thumb.width as f64);
                    let scale_y = icon_size as f64 / (cached_thumb.height as f64);
                    let scale = scale_x.min(scale_y);
                    let transform = Affine::scale_non_uniform(scale, scale)
                        .then_translate(Vec2::new(icon_x, icon_y));
                    if let Some(scene) = graphics.as_scene_mut() {
                        scene.draw_image(&image_brush, transform);
                    }
                    use_thumbnail = true;
                    }
                }
            }

            if !use_thumbnail {
                // Request thumbnail generation if supported
                if self.thumbnail_provider.is_supported(&entry) {
                    let mut pending = self.pending_thumbnails.lock().unwrap();
                    if !pending.contains(&entry.path) {
                        if let Ok(()) = self
                            .thumbnail_provider
                            .request_thumbnail(&entry, thumb_size)
                        {
                            pending.insert(entry.path.clone());
                        }
                    }
                }

                // Get icon for this entry
                let cache_key = (entry.path.clone(), thumb_size);
                let cached_icon = {
                    let mut cache = self.icon_cache.lock().unwrap();
                    if let Some(icon) = cache.get(&cache_key) {
                        icon.clone()
                    } else {
                        let icon = smol::block_on(self.icon_registry.get_file_icon(&entry, thumb_size));
                        cache.insert(cache_key.clone(), icon.clone());
                        icon
                    }
                };

                if let Some(icon) = cached_icon {
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
                            let scale_x = icon_size as f64 / (width as f64);
                            let scale_y = icon_size as f64 / (height as f64);
                            let scale = scale_x.min(scale_y);
                            let transform = Affine::scale_non_uniform(scale, scale)
                                .then_translate(Vec2::new(icon_x, icon_y));
                            if let Some(scene) = graphics.as_scene_mut() {
                                scene.draw_image(&image_brush, transform);
                            }
                        },
                        nptk_services::icon::CachedIcon::Svg(svg_source) => {
                            use vello_svg::usvg::{
                                ImageRendering, Options, ShapeRendering, TextRendering, Tree,
                            };

                            // Check scene cache first
                            let cached_entry =
                                self.svg_scene_cache.get(svg_source.as_str()).cloned();

                            let (scene, svg_width, svg_height) = if let Some(entry) = cached_entry {
                                entry
                            } else {
                                // Parse and render if not in cache
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
                                    let size = tree.size();
                                    let width = size.width() as f64;
                                    let height = size.height() as f64;

                                    self.svg_scene_cache.insert(
                                        svg_source.as_str().to_string(),
                                        (scene.clone(), width, height),
                                    );
                                    (scene, width, height)
                                } else {
                                    (nptk_core::vg::Scene::new(), 48.0, 48.0)
                                }
                            };

                            let scale_x = icon_size as f64 / svg_width;
                            let scale_y = icon_size as f64 / svg_height;
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
                    // Fallback
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

            // 3. Draw Icon Overlay (Selection/Hover)
            if is_selected || is_hovered {
                let color_prop = if is_selected {
                    nptk_theme::properties::ThemeProperty::ColorBackgroundSelected
                } else {
                    nptk_theme::properties::ThemeProperty::ColorMenuHovered
                };

                let color = theme
                    .get_property(self.widget_id(), &color_prop)
                    .or_else(|| theme.get_default_property(&color_prop))
                    .unwrap_or_else(|| {
                        if is_selected {
                            Color::from_rgb8(100, 150, 255)
                        } else {
                            Color::from_rgb8(240, 240, 240)
                        }
                    });

                let alpha = if is_selected { 0.5 } else { 0.3 };

                let icon_overlay_rect =
                    RoundedRect::from_rect(icon_rect, RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0));

                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(alpha)),
                    None,
                    &icon_overlay_rect.to_path(0.1),
                );
            }

            // 4. Draw Label Text
            let text_color = theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorText,
                )
                .or_else(|| {
                    theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText)
                })
                .unwrap_or(Color::BLACK);

            // Use label_rect to position text (reverse padding)
            let text_x = label_rect.x0 + 4.0; // label_padding_x
            let text_y = label_rect.y0 + 2.0; // label_padding_y
            let max_text_width = cell_width - (32.0 + 8.0 * 2.0 + 10.0); // Re-calculate or pass it? Re-calc is cheap.

            let transform = Affine::translate((text_x, text_y));

            // NOTE: Text rendering is the performance bottleneck (~23ms per frame)
            // Both render_text() and render_text_with_max_lines() use expensive Parley layouts
            // Future optimization: implement text layout caching
            self.text_render_context.render_text_with_max_lines(
                &mut info.font_context,
                graphics,
                &entry.name,
                None,
                font_size,
                Brush::Solid(text_color),
                transform,
                true,
                Some(max_text_width as f32),
                Some(2), // Max 2 lines
                false,   // Left align
            );
        }
    }
}
