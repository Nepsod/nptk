// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{FlexWrap, LayoutNode, LayoutStyle, LengthPercentage, StyleNode, LayoutContext};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, BoxedWidget};
use async_trait::async_trait;

/// A wrap container that flows children and wraps them to the next line when needed.
///
/// Similar to Flutter's `Wrap` widget, this container arranges its children
/// in a row and wraps them to the next line when they don't fit.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::wrap::Wrap;
/// use nptk_widgets::text::Text;
///
/// Wrap::new()
///     .with_spacing(10.0)
///     .with_child(Text::new("Item 1"))
///     .with_child(Text::new("Item 2"))
///     .with_child(Text::new("Item 3"))
/// ```
pub struct Wrap {
    children: Vec<BoxedWidget>,
    spacing: Vector2<f32>,
    run_spacing: f32,
}

impl Wrap {
    /// Create a new wrap container.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: Vector2::new(10.0, 10.0),
            run_spacing: 10.0,
        }
    }

    /// Add a child widget to this wrap container.
    pub fn with_child(mut self, child: impl Widget + Send + Sync + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Set the spacing between items (horizontal and vertical).
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = Vector2::new(spacing, spacing);
        self.run_spacing = spacing;
        self
    }

    /// Set the spacing between items (horizontal and vertical separately).
    pub fn with_spacing_xy(mut self, spacing_x: f32, spacing_y: f32) -> Self {
        self.spacing = Vector2::new(spacing_x, spacing_y);
        self.run_spacing = spacing_y;
        self
    }

    /// Set the spacing between runs (lines) of wrapped items.
    pub fn with_run_spacing(mut self, run_spacing: f32) -> Self {
        self.run_spacing = run_spacing;
        self
    }
}

impl Default for Wrap {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for Wrap {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                child.render(graphics, &layout.children[i], info, context.clone());
            }
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: LayoutStyle {
                flex_wrap: FlexWrap::Wrap,
                gap: Vector2::new(
                    LengthPercentage::length(self.spacing.x),
                    LengthPercentage::length(self.run_spacing),
                ),
                ..Default::default()
            },
            children: self.children.iter().map(|c| c.layout_style(context)).collect(),
            measure_func: None,
        }
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        let mut update = Update::empty();
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                update |= child.update(&layout.children[i], context.clone(), info).await;
            }
        }
        update
    }

}
