//! Icon rendering logic for icon widget.

use std::collections::HashMap;

use nptk_core::theme::{ColorRole, Palette};
use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Fill};
use nptk_core::vgi::Graphics;
use npio::service::icon::CachedIcon;

use crate::icon::constants::{FALLBACK_ICON_ALPHA, FALLBACK_ICON_BORDER_RADIUS};

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
    // Optimize cache retrieval: use reference directly to avoid cloning on cache hit
    // nptk_core::vg::Scene is a re-export of vello::Scene, so we can use it directly
    // graphics.append() takes &vello::Scene, and nptk_core::vg::Scene is vello::Scene
    let (vello_scene_ref, svg_width, svg_height) = if let Some((cached_scene, w, h)) = svg_scene_cache.get(svg_source) {
        // Cache hit - use reference to cached scene directly - no cloning needed
        (cached_scene, *w, *h)
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
            // Clone to store in cache
            // Note: We still need to clone on cache miss, but avoid cloning on cache hit (common case)
            svg_scene_cache.insert(
                svg_source.to_string(),
                (scene.clone(), w, h),
            );
            // Get the scene back from cache to use for rendering (avoids lifetime issues)
            // This is a second lookup but only happens on cache miss
            if let Some((cached_scene, _, _)) = svg_scene_cache.get(svg_source) {
                (cached_scene, w, h)
            } else {
                return; // Should not happen, but handle it
            }
        } else {
            return; // Invalid SVG, skip rendering
        }
    };

    let scale_x = icon_size_f64 / svg_width;
    let scale_y = icon_size_f64 / svg_height;
    let scale = scale_x.min(scale_y);
    let transform = Affine::scale_non_uniform(scale, scale)
        .then_translate(Vec2::new(icon_x, icon_y));
    // Append takes &vello::Scene - we have a reference from cache or the newly created scene
    graphics.append(vello_scene_ref, Some(transform));
}

/// Get icon color from palette.
fn get_icon_color(palette: &Palette) -> nptk_core::vg::peniko::Color {
    // Use BaseText or WindowText for icon color
    palette.color(ColorRole::BaseText)
}

/// Render a fallback icon (used when icon is not found/loaded).
pub fn render_fallback_icon(
    graphics: &mut dyn Graphics,
    palette: &Palette,
    icon_rect: Rect,
) {
    let icon_color = get_icon_color(palette);
    let fallback_color = icon_color.with_alpha(FALLBACK_ICON_ALPHA);

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
    palette: &Palette,
    icon: CachedIcon,
    icon_rect: Rect,
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
            // Path variant means async loading is needed - render fallback
            render_fallback_icon(graphics, palette, icon_rect);
        },
    }
}
