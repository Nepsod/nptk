// SPDX-License-Identifier: MIT OR Apache-2.0

//! Text rendering using Parley for proper text layout and glyph mapping

use parley::{FontContext, LayoutContext, StyleProperty, Alignment, Layout};
use parley::fontique::{Collection, CollectionOptions};
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Fill};
use fontique::QueryFont;

/// Brush index type for Parley integration
#[derive(Clone, PartialEq, Default, Debug)]
pub struct BrushIndex(pub usize);

/// Text rendering context that manages font and layout contexts
pub struct TextRenderContext {
    font_cx: FontContext,
    layout_cx: LayoutContext,
}

impl TextRenderContext {
    /// Create a new text rendering context
    pub fn new() -> Self {
        // Create FontContext with system fonts loaded
        let font_cx = FontContext {
            collection: Collection::new(CollectionOptions {
                system_fonts: true,
                ..Default::default()
            }),
            source_cache: Default::default(),
        };
        
        
        Self {
            font_cx,
            layout_cx: LayoutContext::new(),
        }
    }

    /// Render text using Parley for proper layout and glyph mapping
    pub fn render_text(
        &mut self,
        scene: &mut Scene,
        text: &str,
        font: Option<QueryFont>,
        font_size: f32,
        color: Brush,
        transform: Affine,
        hint: bool,
    ) {
        if text.is_empty() {
            return;
        }

        // Try Parley first, but fall back to simple rendering if it fails
        if let Err(_e) = self.try_render_with_parley(scene, text, font.clone(), font_size, color.clone(), transform, hint) {
            self.render_simple_fallback(scene, text, font, font_size, color, transform);
        }
    }

    /// Try to render with Parley
    fn try_render_with_parley(
        &mut self,
        scene: &mut Scene,
        text: &str,
        _font: Option<QueryFont>,
        font_size: f32,
        color: Brush,
        transform: Affine,
        hint: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a text layout using Parley
        let display_scale = 1.0;
        let mut builder = self.layout_cx.ranged_builder(&mut self.font_cx, text, display_scale, true);
        
        // Set font size
        builder.push_default(StyleProperty::FontSize(font_size));
        
        let mut layout = builder.build(text);
        
        // Perform layout operations
        layout.break_all_lines(None);
        layout.align(None, Alignment::Start, Default::default());
        
        
        // Create brushes array
        let brushes = vec![color];
        
        // Render the text using Parley's layout
        self.render_layout_simple(scene, &layout, &brushes, transform, hint);
        
        Ok(())
    }

    /// Simple fallback rendering method
    fn render_simple_fallback(
        &self,
        scene: &mut Scene,
        text: &str,
        _font: Option<QueryFont>,
        font_size: f32,
        color: Brush,
        transform: Affine,
    ) {
        
        // Use system fonts for fallback rendering
        // For now, we'll skip fallback rendering since we have automatic font selection
        // The main Parley rendering should handle most cases with proper font fallback
        log::warn!("Could not render text '{}' - no suitable font available", text);
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
        let mut total_glyphs = 0;
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
                
                // Use the first brush for simple rendering
                let brush = &brushes[0];
                
                let glyphs: Vec<_> = glyph_run.glyphs().collect();
                total_glyphs += glyphs.len();
                
                
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
        }
    }

    /// Measure the width of text using Parley's layout system
    pub fn measure_text_width(&self, text: &str, font_size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        
        // Create a temporary layout context for measurement
        let mut temp_layout_cx = LayoutContext::<[u8; 4]>::new();
        
        // Create a temporary font context for measurement
        let mut temp_font_cx = FontContext {
            collection: self.font_cx.collection.clone(),
            source_cache: Default::default(),
        };
        
        // Create a text layout using Parley to get accurate measurements
        let display_scale = 1.0;
        let mut builder = temp_layout_cx.ranged_builder(&mut temp_font_cx, text, display_scale, true);
        
        // Set font size
        builder.push_default(StyleProperty::FontSize(font_size));
        
        let mut layout = builder.build(text);
        
        // Perform layout operations
        layout.break_all_lines(None);
        layout.align(None, Alignment::Start, Default::default());
        
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
