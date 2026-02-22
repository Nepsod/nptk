//! # Postfix Rendering Pattern - Complete Guide
//!
//! This example demonstrates how to use the postfix rendering pattern to create
//! widgets with overlays, popups, tooltips, and other content that appears "on top".
//!
//! ## What is Postfix Rendering?
//!
//! Postfix rendering is a pattern where widgets can render content AFTER their children
//! have been rendered, ensuring that content appears on top. It provides a simple, robust way to implement overlays
//! without needing a separate overlay management system.
//!
//! ## How to Run This Example
//!
//! ```bash
//! cargo run --example postfix_rendering_guide
//! ```

use nptk::core::app::Application;
use nptk::core::app::context::AppContext;
use nptk::core::app::info::AppInfo;
use nptk::core::app::update::Update;
use nptk::core::layout::{
    AlignItems, Dimension, FlexDirection, LayoutNode, LayoutStyle, LengthPercentage, StyleNode,
};
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Shape};
use vello::peniko::{Brush, Color, Fill};
use vello::Scene;
use nptk::widgets::button::Button;
use nptk::widgets::container::Container;
use nptk::widgets::text::Text;
use nptk::core::widget::{BoxedWidget, Widget, WidgetChildExt, WidgetLayoutExt};
use nptk::core::window::{ElementState, MouseButton};
use nptk::math::Vector2;
use nptk_core::signal::MaybeSignal;
use nptk_core::vgi::vello_vg::VelloGraphics;
use nptk_core::vgi::Graphics;
use async_trait::async_trait;

// ============================================================================
// Example 1: Simple Tooltip Widget
// ============================================================================

/// A widget that shows a tooltip on hover.
///
/// This demonstrates the basic postfix rendering pattern:
/// 1. Render main content in `render()`
/// 2. Render tooltip in `render_postfix()` when hovered
struct TooltipWidget {
    child: BoxedWidget,
    tooltip_text: String,
    is_hovered: bool,
    layout_style: MaybeSignal<LayoutStyle>,
}

impl TooltipWidget {
    fn new(child: impl Widget + 'static, tooltip_text: impl Into<String>) -> Self {
        Self {
            child: Box::new(child),
            tooltip_text: tooltip_text.into(),
            is_hovered: false,
            layout_style: LayoutStyle::default().into(),
        }
    }
}

impl WidgetChildExt for TooltipWidget {
    fn set_child(&mut self, child: impl Widget + 'static) {
        self.child = Box::new(child);
    }
}

impl WidgetLayoutExt for TooltipWidget {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

#[async_trait(?Send)]
impl Widget for TooltipWidget {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Step 1: Render the main widget content (the child) with proper positioning
        if !layout.children.is_empty() {
            let mut child_scene = Scene::new();
            let mut child_graphics = VelloGraphics::new(&mut child_scene);
            self.child.render(
                &mut child_graphics,
                &layout.children[0],
                info,
                context,
            );

            graphics.append(
                &child_scene,
                Some(Affine::translate((
                    layout.layout.location.x as f64,
                    layout.layout.location.y as f64,
                ))),
            );
        }
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Step 2: Render tooltip ON TOP if hovered
        if self.is_hovered {
            // Calculate tooltip position (below and centered)
            let widget_width = layout.layout.size.width as f64;
            let tooltip_width = self.tooltip_text.len() as f64 * 8.0 + 16.0;
            let tooltip_height = 30.0;

            let tooltip_x = layout.layout.location.x as f64 + (widget_width - tooltip_width) / 2.0;
            let tooltip_y =
                layout.layout.location.y as f64 + layout.layout.size.height as f64 + 5.0;

            // Draw tooltip background
            let tooltip_rect = RoundedRect::new(
                tooltip_x,
                tooltip_y,
                tooltip_x + tooltip_width,
                tooltip_y + tooltip_height,
                RoundedRectRadii::from_single_radius(4.0),
            );

            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(Color::from_rgba8(40, 40, 40, 230)),
                None,
                &tooltip_rect.to_path(0.1),
            );

            // Draw tooltip text
            use nptk_core::text_render::TextRenderContext;
            let mut text_ctx = TextRenderContext::new();
            text_ctx.render_text(
                &mut info.font_context,
                graphics,
                &self.tooltip_text,
                None,
                14.0,
                Brush::Solid(Color::WHITE),
                Affine::translate((tooltip_x + 8.0, tooltip_y + 8.0 + 14.0)),
                true,
                None,
            );
        }
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Update child first
        if !layout.children.is_empty() {
            update |= self.child.update(&layout.children[0], context, info).await;
        }

        // Check if mouse is hovering over widget
        let old_hovered = self.is_hovered;
        self.is_hovered = false;

        if let Some(cursor_pos) = info.cursor_pos {
            let bounds = Rect::new(
                layout.layout.location.x as f64,
                layout.layout.location.y as f64,
                (layout.layout.location.x + layout.layout.size.width) as f64,
                (layout.layout.location.y + layout.layout.size.height) as f64,
            );

            if bounds.contains((cursor_pos.x, cursor_pos.y)) {
                self.is_hovered = true;
            }
        }

        // Request redraw if hover state changed
        if old_hovered != self.is_hovered {
            update |= Update::DRAW;
        }

        update
    }

    fn layout_style(&self, _context: &nptk::core::layout::LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.child.layout_style(_context)],
            measure_func: None,
        }
    }
}

// ============================================================================
// Example 2: Simple Dropdown Widget
// ============================================================================

/// A simple dropdown widget that shows a list of options when clicked.
///
/// This demonstrates:
/// 1. Rendering a button in `render()`
/// 2. Rendering the dropdown list in `render_postfix()` when open
/// 3. Handling click-outside-to-close logic
struct SimpleDropdown {
    items: Vec<String>,
    selected_index: usize,
    is_open: bool,
    layout_style: MaybeSignal<LayoutStyle>,
}

impl SimpleDropdown {
    fn new(items: Vec<String>) -> Self {
        Self {
            selected_index: 0,
            is_open: false,
            items,
            layout_style: LayoutStyle::default().into(),
        }
    }
}

impl WidgetLayoutExt for SimpleDropdown {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

#[async_trait(?Send)]
impl Widget for SimpleDropdown {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Step 1: Render the button showing selected item
        let button_rect = RoundedRect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
            RoundedRectRadii::from_single_radius(4.0),
        );

        let color = if self.is_open {
            Color::from_rgb8(100, 150, 200)
        } else {
            Color::from_rgb8(150, 150, 150)
        };

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(color),
            None,
            &button_rect.to_path(0.1),
        );

        // Draw selected item text
        // Text baseline renders at y + font_size, so we need to position accordingly
        use nptk_core::text_render::TextRenderContext;
        let mut text_ctx = TextRenderContext::new();
        let font_size = 16.0_f32;
        let _button_height = layout.layout.size.height as f64;
        text_ctx.render_text(
            &mut info.font_context,
            graphics,
            &self.items[self.selected_index],
            None,
            font_size,
            Brush::Solid(Color::WHITE),
            Affine::translate((
                layout.layout.location.x as f64 + 10.0,
                layout.layout.location.y as f64 + font_size as f64,
            )),
            true,
            None,
        );
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Step 2: Render dropdown list ON TOP when open
        if !self.is_open {
            return;
        }

        let item_height = 30.0;
        let list_width = layout.layout.size.width as f64;
        let list_height = self.items.len() as f64 * item_height;

        let list_x = layout.layout.location.x as f64;
        let list_y = layout.layout.location.y as f64 + layout.layout.size.height as f64 + 2.0;

        // Draw list background
        let list_rect = RoundedRect::new(
            list_x,
            list_y,
            list_x + list_width,
            list_y + list_height,
            RoundedRectRadii::from_single_radius(4.0),
        );

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(Color::from_rgb8(240, 240, 240)),
            None,
            &list_rect.to_path(0.1),
        );

        // Draw each item
        use nptk_core::text_render::TextRenderContext;
        let mut text_ctx = TextRenderContext::new();

        for (i, item) in self.items.iter().enumerate() {
            let item_y = list_y + (i as f64 * item_height);

            // Highlight if selected
            if i == self.selected_index {
                let highlight_rect =
                    Rect::new(list_x, item_y, list_x + list_width, item_y + item_height);
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgb8(200, 220, 240)),
                    None,
                    &highlight_rect.to_path(0.1),
                );
            }

            // Draw item text
            // Text baseline renders at y + font_size
            let item_font_size = 16.0_f32;
            let _y_offset = (item_height - item_font_size as f64) / 2.0 + item_font_size as f64;
            text_ctx.render_text(
                &mut info.font_context,
                graphics,
                item,
                None,
                item_font_size,
                Brush::Solid(Color::BLACK),
                Affine::translate((list_x + 10.0, item_y + 8.0)),
                true,
                None,
            );
        }
    }

    async fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        let cursor_pos = info.cursor_pos;

        // Handle button click to toggle dropdown
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                if let Some(pos) = cursor_pos {
                    let button_rect = Rect::new(
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64,
                        (layout.layout.location.x + layout.layout.size.width) as f64,
                        (layout.layout.location.y + layout.layout.size.height) as f64,
                    );

                    if button_rect.contains((pos.x, pos.y)) {
                        self.is_open = !self.is_open;
                        update |= Update::DRAW;
                    } else if self.is_open {
                        // Click outside - close dropdown
                        self.is_open = false;
                        update |= Update::DRAW;
                    }
                }
            }
        }

        update
    }

    fn layout_style(&self, _context: &nptk::core::layout::LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![],
            measure_func: None,
        }
    }
}

// ============================================================================
// Main Application
// ============================================================================

struct PostfixGuideApp;

impl Application for PostfixGuideApp {
    type State = ();

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        Container::new(vec![
            // Title
            Box::new(
                Text::new("Postfix Rendering Examples".to_string()).with_layout_style(
                    LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                        ..Default::default()
                    },
                ),
            ),
            // Subtitle
            Box::new(
                Text::new("Hover over the button for a tooltip, click the dropdown".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                        ..Default::default()
                    }),
            ),
            // Example 1: Tooltip
            Box::new(
                TooltipWidget::new(
                    Button::new(Text::new("Hover Me!".to_string())).with_layout_style(
                        LayoutStyle {
                            size: Vector2::new(Dimension::length(120.0), Dimension::length(40.0)),
                            ..Default::default()
                        },
                    ),
                    "This is a tooltip rendered in postfix!",
                )
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::auto(), Dimension::auto()),
                    ..Default::default()
                }),
            ),
            // Example 2: Dropdown
            Box::new(
                SimpleDropdown::new(vec![
                    "Option 1".to_string(),
                    "Option 2".to_string(),
                    "Option 3".to_string(),
                    "Option 4".to_string(),
                ])
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::length(200.0), Dimension::length(40.0)),
                    ..Default::default()
                }),
            ),
            // Instructions
            Box::new(
                Text::new(
                    "Both the tooltip and dropdown use render_postfix() to appear on top!"
                        .to_string(),
                )
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::percent(1.0), Dimension::auto()),
                    ..Default::default()
                }),
            ),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            gap: Vector2::new(
                LengthPercentage::length(0.0),
                LengthPercentage::length(20.0),
            ),
            padding: nptk_core::layout::Rect::<LengthPercentage> {
                left: LengthPercentage::length(20.0),
                right: LengthPercentage::length(20.0),
                top: LengthPercentage::length(40.0),
                bottom: LengthPercentage::length(20.0),
            },
            ..Default::default()
        })
    }
}

fn main() {
    PostfixGuideApp.run(());
}
