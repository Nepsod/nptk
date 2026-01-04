//! Icon rendering logic for file icon widget.

use std::collections::HashMap;

use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Fill};
use nptk_core::vgi::Graphics;
use npio::service::icon::CachedIcon;
use nptk_services::filesystem::entry::{FileEntry, FileType};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use vello_svg;

use crate::file_icon::constants::{DIRECTORY_FALLBACK_ALPHA, FALLBACK_ICON_BORDER_RADIUS, FILE_FALLBACK_ALPHA};
use crate::file_icon::theme::get_icon_color;

/// Render an image icon.
pub fn render_image_icon(
    graphics: &mut dyn Graphics,
    data: &[u8],
    width: u32,
    height: u32,
    icon_rect: Rect,
) {
    use nptk_core::vg::peniko::{
        Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
    };
    
    let icon_x = icon_rect.x0;
    let icon_y = icon_rect.y0;
    let icon_size_f64 = icon_rect.width().min(icon_rect.height());

    let image_data = ImageData {
        data: Blob::from(data.to_vec()),
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
}

/// Render an SVG icon with caching support.
pub fn render_svg_icon(
    graphics: &mut dyn Graphics,
    svg_source: &str,
    icon_rect: Rect,
    svg_scene_cache: &mut HashMap<String, (nptk_core::vg::Scene, f64, f64)>,
) {
    let icon_x = icon_rect.x0;
    let icon_y = icon_rect.y0;
    let icon_size_f64 = icon_rect.width().min(icon_rect.height());

    // Check SVG scene cache first
    let cached_scene = svg_scene_cache.get(svg_source).cloned();
    let (scene, svg_width, svg_height) = if let Some((scene, w, h)) = cached_scene {
        (scene, w, h)
    } else {
        // Cache miss - parse and render SVG
        use vello_svg::usvg::{
            ImageRendering, Options, ShapeRendering, TextRendering, Tree,
        };
        if let Ok(tree) = Tree::from_str(
            svg_source,
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
            svg_scene_cache.insert(
                svg_source.to_string(),
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
}

/// Render a fallback icon (used when icon path is unavailable or icon not yet loaded).
pub fn render_fallback_icon(
    graphics: &mut dyn Graphics,
    theme: &mut dyn Theme,
    widget_id: WidgetId,
    icon_rect: Rect,
    entry: &FileEntry,
) {
    let icon_color = get_icon_color(theme, widget_id);
    let is_directory = entry.file_type == FileType::Directory;
    let alpha = if is_directory {
        DIRECTORY_FALLBACK_ALPHA
    } else {
        FILE_FALLBACK_ALPHA
    };
    let fallback_color = icon_color.with_alpha(alpha);

    graphics.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        &Brush::Solid(fallback_color),
        None,
        &icon_rect.to_path(FALLBACK_ICON_BORDER_RADIUS),
    );
}

/// Render a cached icon based on its type.
pub fn render_cached_icon(
    graphics: &mut dyn Graphics,
    theme: &mut dyn Theme,
    widget_id: WidgetId,
    icon: CachedIcon,
    icon_rect: Rect,
    entry: &FileEntry,
    svg_scene_cache: &mut HashMap<String, (nptk_core::vg::Scene, f64, f64)>,
) {
    match icon {
        CachedIcon::Image {
            data,
            width,
            height,
        } => {
            render_image_icon(graphics, data.as_ref(), width, height, icon_rect);
        },
        CachedIcon::Svg(svg_source) => {
            render_svg_icon(graphics, &svg_source, icon_rect, svg_scene_cache);
        },
        CachedIcon::Path(_) => {
            render_fallback_icon(graphics, theme, widget_id, icon_rect, entry);
        },
    }
}

/// Render SVG icon with thread-safe cache (for properties widget).
pub fn render_svg_icon_with_arc_cache(
    graphics: &mut dyn Graphics,
    svg_source: &str,
    icon_rect: Rect,
    svg_scene_cache: &std::sync::Arc<std::sync::Mutex<HashMap<String, (nptk_core::vg::Scene, f64, f64)>>>,
) {
    let mut cache = svg_scene_cache.lock().unwrap();
    render_svg_icon(graphics, svg_source, icon_rect, &mut *cache);
}
