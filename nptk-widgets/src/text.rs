use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::peniko::Brush;
use nptk_core::vg::{Scene};
use nptk_core::text_render::TextRenderContext;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;
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

impl Widget for Text {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        _: AppContext,
    ) {
        let font_size = *self.font_size.get();
        let hinting = *self.hinting.get();
        let text = self.text.get();

        let color = if let Some(style) = theme.of(Self::widget_id(self)) {
            if theme.globals().invert_text_color {
                style.get_color("color_invert").unwrap()
            } else {
                style.get_color("color").unwrap()
            }
        } else {
            theme.defaults().text().foreground()
        };
        
        log::debug!("Text widget rendering: '{}' with color: {:?} at position: ({}, {})", 
                   *text, color, layout_node.layout.location.x, layout_node.layout.location.y);

        // Use TextRenderContext for proper text rendering
        let transform = nptk_core::vg::kurbo::Affine::translate((
            layout_node.layout.location.x as f64,
            layout_node.layout.location.y as f64 + font_size as f64,
        ));
        
        self.text_render_context.render_text(
            &mut info.font_context,
            scene,
            text.as_ref(),
            None, // No specific font, use default
            font_size,
            Brush::Solid(color),
            transform,
            hinting,
        );
        
    }

    fn layout_style(&self) -> StyleNode {
        let text = self.text.get();
        let font_size = *self.font_size.get();
        let line_gap = *self.line_gap.get();

        // A simple approximation for line height
        let line_height = font_size + line_gap;
        let line_count = text.lines().count().max(1) as f32;
        let calculated_height = line_height * line_count;

        let style = self.style.get().deref().clone();

        StyleNode {
            style: LayoutStyle {
                size: Vector2::new(
                    style.size.x, // Keep user-defined width or default
                    Dimension::length(calculated_height),
                ),
                ..style
            },
            children: Vec::new(),
        }
    }

    fn update(&mut self, _: &LayoutNode, _: AppContext, _: &mut AppInfo) -> Update {
        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Text")
    }
}
