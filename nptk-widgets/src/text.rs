use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{AvailableSpace, Dimension, LayoutContext, LayoutNode, LayoutStyle, Size, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::peniko::{Brush, Color};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use async_trait::async_trait;
use std::ops::Deref;

/// Displays the given text with optional font, size and hinting.
///
/// ### Theming
/// You can style the text with the following properties:
/// - `color` - The color of the text.
/// - `color_invert` - The color to use when the `invert_color` property is set to `true` in the theme [Globals].
///
/// [Globals]: nptk_theme::globals::Globals
pub struct Text {
    style: MaybeSignal<LayoutStyle>,
    text: MaybeSignal<String>,
    font: MaybeSignal<Option<String>>,
    font_size: MaybeSignal<f32>,
    hinting: MaybeSignal<bool>,
    line_gap: MaybeSignal<f32>,
    text_render_context: TextRenderContext,
    measured_size: Option<Vector2<f32>>,
}

impl Text {
    /// Create a new text widget with the given text.
    pub fn new(text: impl Into<MaybeSignal<String>>) -> Self {
        Self {
            style: LayoutStyle::default().into(),
            text: text.into(),
            font: None.into(),
            font_size: 30.0.into(),
            hinting: true.into(),
            line_gap: 7.5.into(),
            text_render_context: TextRenderContext::new(),
            measured_size: None,
        }
    }

    /// Set the hinting of the text.
    ///
    /// Hinting adjusts the display of an outline font so that it lines up with a rasterized grid.
    /// At low screen resolutions and font size, hinting can produce clearer text.
    pub fn with_hinting(mut self, hinting: impl Into<MaybeSignal<bool>>) -> Self {
        self.hinting = hinting.into();
        self
    }

    /// Set the font of the text.
    pub fn with_font(mut self, font: impl Into<MaybeSignal<Option<String>>>) -> Self {
        self.font = font.into();
        self
    }

    /// Set the font size of the text.
    pub fn with_font_size(mut self, size: impl Into<MaybeSignal<f32>>) -> Self {
        self.font_size = size.into();
        self
    }

    /// Set the line gap of the text.
    ///
    /// The line gap is the space between lines of text. Defaults to `7.5`.
    pub fn with_line_gap(mut self, gap: impl Into<MaybeSignal<f32>>) -> Self {
        self.line_gap = gap.into();
        self
    }
}

impl WidgetLayoutExt for Text {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.style = layout_style.into();
    }
}

#[async_trait(?Send)]
impl Widget for Text {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        let font_size = *self.font_size.get();
        let hinting = *self.hinting.get();
        let text = self.text.get();
        let font_name = self.font.get().clone();

        let palette = context.palette();
        
        // Use palette-based color selection
        // Default to BaseText for text in Base background, WindowText for Window background
        // For now, use BaseText as default (can be enhanced later to check parent background)
        let color = palette.color(nptk_core::theme::ColorRole::BaseText);

        // Use TextRenderContext for proper text rendering
        let transform = nptk_core::vg::kurbo::Affine::translate((
            layout_node.layout.location.x as f64,
            layout_node.layout.location.y as f64,
        ));

        // Get the available width for text wrapping
        // The layout size is already computed by Taffy and accounts for padding
        // Taffy's layout.size.width is the content area width (after padding)
        let max_width = if layout_node.layout.size.width > 0.0 {
            Some(layout_node.layout.size.width)
        } else {
            // Fallback: if width is 0, text won't wrap
            // This can happen if Taffy hasn't computed width yet or widget is in a flex container
            log::warn!(
                "Text widget has zero width, text will not wrap. Layout: {:?}",
                layout_node.layout
            );
            None
        };

        self.text_render_context.render_text(
            &mut info.font_context,
            graphics,
            text.as_ref(),
            font_name,
            font_size,
            Brush::Solid(color),
            transform,
            hinting,
            max_width,
        );
    }

    fn measure(&self, constraints: Size<AvailableSpace>) -> Option<Size<f32>> {
        let text = self.text.get();
        let font_size = *self.font_size.get();
        let line_gap = *self.line_gap.get();

        // Get max width from constraints
        let max_width = match constraints.width {
            AvailableSpace::Definite(w) if w > 0.0 => Some(w),
            AvailableSpace::Definite(_) => None, // Zero or negative width
            AvailableSpace::MinContent => None,
            AvailableSpace::MaxContent => None,
        };

        // Use measured size if available and constraints allow
        if let Some(measured) = self.measured_size {
            if let Some(max_w) = max_width {
                if measured.x <= max_w {
                    return Some(Size {
                        width: measured.x,
                        height: measured.y,
                    });
                }
                // If measured width exceeds max, we need to recalculate with wrapping
            } else {
                return Some(Size {
                    width: measured.x,
                    height: measured.y,
                });
            }
        }

        // Calculate line height (font size + line gap)
        let line_height = font_size + line_gap;

        // Estimate text width and height
        // For better accuracy, we estimate based on character count and average character width
        // Average character width is typically 0.6-0.7 of font size for most fonts
        let avg_char_width = font_size * 0.65;
        let char_count = text.chars().count();

        let (width, line_count) = if let Some(max_w) = max_width {
            // With width constraint, estimate wrapping
            // Estimate characters per line based on max width
            let chars_per_line = (max_w / avg_char_width).max(1.0) as usize;
            let estimated_lines = ((char_count + chars_per_line - 1) / chars_per_line).max(1);
            
            // Width is the minimum of: full text width or max width
            let full_width = char_count as f32 * avg_char_width;
            (full_width.min(max_w), estimated_lines)
        } else {
            // No width constraint, estimate single line (or natural wrapping)
            // For MaxContent, we want the full width
            let full_width = char_count as f32 * avg_char_width;
            let natural_lines = text.lines().count().max(1);
            (full_width, natural_lines)
        };

        let height = line_height * line_count as f32;

        Some(Size { width, height })
    }

    fn layout_style(&self, _context: &LayoutContext) -> StyleNode {
        let text = self.text.get();
        let font_size = *self.font_size.get();
        let line_gap = *self.line_gap.get();

        // A simple approximation for line height
        let line_height = font_size + line_gap;
        let line_count = text.lines().count().max(1) as f32;
        let calculated_height = line_height * line_count;

        let style = self.style.get().deref().clone();

        // Use measure function if available for better sizing
        let width = if style.size.x == Dimension::auto() {
            if let Some(measured) = self.measured_size {
                Dimension::length(measured.x)
            } else {
                // Try to use measure function with unbounded constraints
                if let Some(size) = self.measure(Size {
                    width: AvailableSpace::MaxContent,
                    height: AvailableSpace::MaxContent,
                }) {
                    Dimension::length(size.width)
                } else {
                    // Heuristic: estimate width based on char count
                    let char_count = text.chars().count();
                    let estimated_width = char_count as f32 * font_size * 0.8;
                    Dimension::length(estimated_width)
                }
            }
        } else {
            style.size.x // Keep user-defined width
        };

        StyleNode {
            style: LayoutStyle {
                size: Vector2::new(width, Dimension::length(calculated_height)),
                flex_grow: if style.size.x == Dimension::auto() {
                    0.0 // Do not grow by default, let content define size
                } else {
                    style.flex_grow
                },
                ..style
            },
            children: Vec::new(),
            measure_func: None, // Text widget uses measure() in layout_style() instead
        }
    }

    async fn update(&mut self, _layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let text = self.text.get();
        let font_size = *self.font_size.get();
        let line_gap = *self.line_gap.get();
        
        let font_name = self.font.get().clone();

        // Measure text
        let (width, line_count) = self.text_render_context.measure_text_layout(
            &mut info.font_context,
            &text,
            font_name,
            font_size,
            None, // No max width constraint for auto-sizing
        );
        
        let line_height = font_size + line_gap;
        let height = line_height * line_count as f32;
        let new_size = Vector2::new(width, height);

        if self.measured_size != Some(new_size) {
            self.measured_size = Some(new_size);
            return Update::LAYOUT | Update::DRAW;
        }

        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Text")
    }
}
