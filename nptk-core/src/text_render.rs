// SPDX-License-Identifier: MIT OR Apache-2.0

//! Text rendering using Parley for proper text layout and glyph mapping

use crate::app::font_ctx::FontContext;
use crate::vgi::Graphics;
use lru::LruCache;
use parley::{Alignment, Layout, LayoutContext, StyleProperty};
use std::num::NonZeroUsize;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Fill};
use vello::Scene;

/// Maximum number of text layouts to cache
const LAYOUT_CACHE_CAPACITY: usize = 1000;

/// Brush index type for Parley integration
#[derive(Clone, PartialEq, Default, Debug)]
pub struct BrushIndex(pub usize);

/// Statistics for text layout cache performance tracking
#[derive(Debug, Default, Clone)]
pub struct TextCacheStats {
    /// Number of cache hits
    pub hits: usize,
    /// Number of cache misses
    pub misses: usize,
    /// Total number of cache lookups
    pub total_lookups: usize,
}

impl TextCacheStats {
    /// Get cache hit ratio (0.0 to 1.0)
    pub fn hit_ratio(&self) -> f64 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        self.hits as f64 / self.total_lookups as f64
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.total_lookups = 0;
    }
}

use std::sync::Mutex;

struct TextRenderInternal {
    layout_cx: LayoutContext,
    layout_cache: LruCache<u64, Layout<[u8; 4]>>,
    cache_stats: TextCacheStats,
}

/// Text rendering context that manages layout contexts
pub struct TextRenderContext {
    internal: Mutex<TextRenderInternal>,
    /// Text direction for RTL support (placeholder for future implementation)
    /// TODO: Integrate with system locale detection and Parley's RTL support
    _text_direction: Option<crate::layout::LayoutDirection>,
}

impl TextRenderContext {
    /// Create a new text rendering context
    pub fn new() -> Self {
        Self {
            internal: Mutex::new(TextRenderInternal {
                layout_cx: LayoutContext::new(),
                layout_cache: LruCache::new(NonZeroUsize::new(LAYOUT_CACHE_CAPACITY).unwrap()),
                cache_stats: TextCacheStats::default(),
            }),
            _text_direction: None, // TODO: Detect from system locale
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> TextCacheStats {
        self.internal.lock().unwrap().cache_stats.clone()
    }

    /// Reset cache statistics
    pub fn reset_stats(&mut self) {
        self.internal.lock().unwrap().cache_stats.reset();
    }

    /// Set text direction for RTL support (placeholder)
    ///
    /// This is a placeholder for future RTL text rendering support.
    /// Full implementation requires integration with Parley's RTL text layout.
    pub fn set_text_direction(&mut self, _direction: crate::layout::LayoutDirection) {
        // TODO: Implement RTL text rendering with Parley
        // For now, this is a no-op placeholder
    }

    /// Render text using Parley for proper layout and glyph mapping
    ///
    /// # Arguments
    /// * `max_width` - Optional maximum width for text wrapping. If None, text will not wrap.
    pub fn render_text(
        &mut self,
        font_cx: &mut FontContext,
        graphics: &mut dyn Graphics,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
        color: Brush,
        transform: Affine,
        hint: bool,
        max_width: Option<f32>,
    ) {
        if text.is_empty() {
            return;
        }

        // Extract Scene from Graphics for Parley rendering
        // Parley needs direct Scene access for glyph drawing
        if let Some(scene) = graphics.as_scene_mut() {
            // Try Parley first, but fall back to simple rendering if it fails
            if let Err(_e) = self.try_render_with_parley(
                font_cx,
                scene,
                text,
                font_family.clone(),
                font_size,
                color.clone(),
                transform,
                hint,
                max_width,
            ) {
                log::debug!("Parley rendering failed, using simple fallback");
                self.render_simple_fallback(
                    font_cx, scene, text, font_family, font_size, color, transform,
                );
            }
        } else {
            log::warn!("Graphics backend does not support text rendering via Parley");
        }
    }

    /// Try to render with Parley
    fn try_render_with_parley(
        &mut self,
        font_cx: &mut FontContext,
        scene: &mut Scene,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
        color: Brush,
        transform: Affine,
        hint: bool,
        max_width: Option<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let layout = self.fetch_layout(
            font_cx,
            text,
            font_family,
            font_size,
            max_width,
            None,
            false, // center_align = false for render_text
        );

        let brushes = vec![color];
        self.render_layout_simple(scene, &layout, &brushes, transform, hint);

        Ok(())
    }

    /// Render text with optional line limit (for truncation)
    pub fn render_text_with_max_lines(
        &mut self,
        font_cx: &mut FontContext,
        graphics: &mut dyn Graphics,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
        color: Brush,
        transform: Affine,
        hint: bool,
        max_width: Option<f32>,
        max_lines: Option<usize>,
        center_align: bool,
    ) {
        if text.is_empty() {
            return;
        }

        // Extract Scene from Graphics for Parley rendering
        if let Some(scene) = graphics.as_scene_mut() {
            // Try Parley first, but fall back to simple rendering if it fails
            if let Err(_e) = self.try_render_with_parley_max_lines(
                font_cx,
                scene,
                text,
                font_family.clone(),
                font_size,
                color.clone(),
                transform,
                hint,
                max_width,
                max_lines,
                center_align,
            ) {
                log::debug!("Parley rendering failed, using simple fallback");
                self.render_simple_fallback(
                    font_cx, scene, text, font_family, font_size, color, transform,
                );
            }
        } else {
            log::warn!("Graphics backend does not support text rendering via Parley");
        }
    }

    /// Try to render with Parley with line limit
    fn try_render_with_parley_max_lines(
        &mut self,
        font_cx: &mut FontContext,
        scene: &mut Scene,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
        color: Brush,
        transform: Affine,
        hint: bool,
        max_width: Option<f32>,
        max_lines: Option<usize>,
        center_align: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let layout = self.fetch_layout(
            font_cx,
            text,
            font_family,
            font_size,
            max_width,
            max_lines,
            center_align,
        );

        let brushes = vec![color];
        self.render_layout_simple_with_max_lines(
            scene, &layout, &brushes, transform, hint, max_lines,
        );

        Ok(())
    }

    /// Simple fallback rendering method
    fn render_simple_fallback(
        &self,
        _font_cx: &mut FontContext,
        _scene: &mut Scene,
        text: &str,
        _font_family: Option<String>,
        _font_size: f32,
        _color: Brush,
        _transform: Affine,
    ) {
        // Use system fonts for fallback rendering
        // For now, we'll skip fallback rendering since we have automatic font selection
        // The main Parley rendering should handle most cases with proper font fallback
        log::warn!(
            "Could not render text '{}' - no suitable font available",
            text
        );
    }

    /// Render a simple Parley layout to the scene (for default layout type)
    fn render_layout_simple(
        &self,
        scene: &mut Scene,
        layout: &Layout<[u8; 4]>,
        brushes: &[Brush],
        transform: Affine,
        hint: bool,
    ) {
        self.render_layout_simple_with_max_lines(scene, layout, brushes, transform, hint, None)
    }

    /// Render layout with optional line limit
    fn render_layout_simple_with_max_lines(
        &self,
        scene: &mut Scene,
        layout: &Layout<[u8; 4]>,
        brushes: &[Brush],
        transform: Affine,
        hint: bool,
        max_lines: Option<usize>,
    ) {
        let _total_glyphs = 0;
        let mut line_index = 0;
        for line in layout.lines() {
            // Stop rendering if we've reached the max lines
            // CRITICAL: This must break BEFORE rendering the line to prevent overflow
            if let Some(max) = max_lines {
                if line_index >= max {
                    // We've reached the max lines, stop rendering
                    break;
                }
            }
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };

                let mut x = glyph_run.offset();
                let y = glyph_run.baseline();
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let coords = run.normalized_coords();

                // Use the first brush for simple rendering
                let brush = &brushes[0];

                let glyphs: Vec<_> = glyph_run.glyphs().collect();
                let _glyph_count = glyphs.len();

                if !glyphs.is_empty() {
                    scene
                        .draw_glyphs(font)
                        .brush(brush)
                        .hint(hint)
                        .transform(transform)
                        .glyph_transform(glyph_xform)
                        .font_size(font_size)
                        .normalized_coords(coords)
                        .draw(
                            Fill::NonZero,
                            glyphs.into_iter().map(|glyph| {
                                let gx = x + glyph.x;
                                let gy = y - glyph.y;
                                x += glyph.advance;
                                vello::Glyph {
                                    id: glyph.id as _,
                                    x: gx,
                                    y: gy,
                                }
                            }),
                        );
                }
            }
            line_index += 1;
        }
    }

    fn fetch_layout(
        &self,
        font_cx: &mut FontContext,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
        max_width: Option<f32>,
        max_lines: Option<usize>,
        center_align: bool,
    ) -> Layout<[u8; 4]> {
        // Use hash-based cache key to avoid string allocations
        // Hash string references directly without cloning
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        font_family.as_deref().hash(&mut hasher);
        max_width.map(|w| w as u32).hash(&mut hasher);
        (font_size as u32).hash(&mut hasher);
        max_lines.hash(&mut hasher);
        center_align.hash(&mut hasher);
        let cache_key = hasher.finish();

        let mut internal = self.internal.lock().unwrap();

        // Track cache lookup
        internal.cache_stats.total_lookups += 1;

        let cached_layout = internal.layout_cache.get(&cache_key).cloned();
        if let Some(layout) = cached_layout {
            // Cache hit
            internal.cache_stats.hits += 1;
            return layout;
        }

        // Cache miss
        internal.cache_stats.misses += 1;

        let display_scale = 1.0;
        let mut parley_font_cx = font_cx.create_parley_font_context();
        let mut builder = internal
            .layout_cx
            .ranged_builder(&mut parley_font_cx, text, display_scale, true);

        builder.push_default(StyleProperty::FontSize(font_size));
        if let Some(family) = font_family {
            builder.push_default(StyleProperty::FontStack(parley::style::FontStack::Single(
                parley::style::FontFamily::Named(std::borrow::Cow::Owned(family)),
            )));
        }

        let mut layout = builder.build(text);

        if let Some(width) = max_width {
            layout.break_all_lines(Some(width));
        } else {
            layout.break_all_lines(None);
        }

        let align = if center_align {
            Alignment::Center
        } else {
            Alignment::Start
        };
        layout.align(max_width, align, Default::default());

        internal.layout_cache.put(cache_key, layout.clone());
        layout
    }

    /// Measure the width of text using Parley's layout system
    pub fn measure_text_width(
        &self,
        font_cx: &mut FontContext,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
    ) -> f32 {
        if text.is_empty() {
            return 0.0;
        }

        let layout = self.fetch_layout(
            font_cx,
            text,
            font_family,
            font_size,
            None,
            None,
            false,
        );

        // Calculate total width by summing up glyph advances
        let mut total_width = 0.0;
        for line in layout.lines() {
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };

                // Sum up all glyph advances in this run
                for glyph in glyph_run.glyphs() {
                    total_width += glyph.advance;
                }
            }
        }

        total_width
    }

    /// Measure text layout and get line count when wrapped to a specific width
    pub fn measure_text_layout(
        &self,
        font_cx: &mut FontContext,
        text: &str,
        font_family: Option<String>,
        font_size: f32,
        max_width: Option<f32>,
    ) -> (f32, usize) {
        if text.is_empty() {
            return (0.0, 0);
        }

        let layout = self.fetch_layout(
            font_cx,
            text,
            font_family,
            font_size,
            max_width,
            None,
            false,
        );

        // Count lines and calculate max width
        let mut line_count = 0;
        let mut max_line_width = 0.0f32;
        for line in layout.lines() {
            line_count += 1;
            let mut line_width = 0.0f32;
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                for glyph in glyph_run.glyphs() {
                    line_width += glyph.advance;
                }
            }
            max_line_width = max_line_width.max(line_width);
        }

        (max_line_width, line_count)
    }

    /// Render a Parley layout to the scene
    pub fn render_layout(
        &self,
        scene: &mut Scene,
        layout: &Layout<BrushIndex>,
        brushes: &[Brush],
        transform: Affine,
        hint: bool,
    ) {
        for line in layout.lines() {
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };

                let mut x = glyph_run.offset();
                let y = glyph_run.baseline();
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let coords = run.normalized_coords();
                let style = glyph_run.style();
                let brush = &brushes[style.brush.0];

                scene
                    .draw_glyphs(font)
                    .brush(brush)
                    .hint(hint)
                    .transform(transform)
                    .glyph_transform(glyph_xform)
                    .font_size(font_size)
                    .normalized_coords(coords)
                    .draw(
                        Fill::NonZero,
                        glyph_run.glyphs().map(|glyph| {
                            let gx = x + glyph.x;
                            let gy = y - glyph.y;
                            x += glyph.advance;
                            vello::Glyph {
                                id: glyph.id as _,
                                x: gx,
                                y: gy,
                            }
                        }),
                    );
            }
        }
    }
}

impl Default for TextRenderContext {
    fn default() -> Self {
        Self::new()
    }
}
