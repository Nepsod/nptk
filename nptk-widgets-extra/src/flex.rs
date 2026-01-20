// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, FlexDirection, LayoutNode, LayoutStyle, StyleNode, LayoutContext};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, BoxedWidget};
use nptk_theme::id::WidgetId;
use async_trait::async_trait;

/// A horizontal stack container that arranges children in a row.
///
/// Similar to SwiftUI's `HStack`, this widget arranges its children
/// horizontally with configurable spacing and alignment.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::flex::HStack;
/// use nptk_widgets::text::Text;
///
/// HStack::new()
///     .with_child(Text::new("Left"))
///     .with_child(Text::new("Right"))
/// ```
pub struct HStack {
    children: Vec<BoxedWidget>,
    gap: f32,
}

impl HStack {
    /// Create a new horizontal stack.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: 0.0,
        }
    }

    /// Add a child widget to this stack.
    pub fn with_child(mut self, child: impl Widget + Send + Sync + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Set the gap between children.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl Default for HStack {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for HStack {
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
                flex_direction: FlexDirection::Row,
                gap: Vector2::new(
                    nptk_core::layout::LengthPercentage::length(self.gap),
                    nptk_core::layout::LengthPercentage::length(0.0),
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

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets-extra", "HStack")
    }
}

/// A vertical stack container that arranges children in a column.
///
/// Similar to SwiftUI's `VStack`, this widget arranges its children
/// vertically with configurable spacing and alignment.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::flex::VStack;
/// use nptk_widgets::text::Text;
///
/// VStack::new()
///     .with_child(Text::new("Top"))
///     .with_child(Text::new("Bottom"))
/// ```
pub struct VStack {
    children: Vec<BoxedWidget>,
    gap: f32,
}

impl VStack {
    /// Create a new vertical stack.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: 0.0,
        }
    }

    /// Add a child widget to this stack.
    pub fn with_child(mut self, child: impl Widget + Send + Sync + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Set the gap between children.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl Default for VStack {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for VStack {
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
                flex_direction: FlexDirection::Column,
                gap: Vector2::new(
                    nptk_core::layout::LengthPercentage::length(0.0),
                    nptk_core::layout::LengthPercentage::length(self.gap),
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

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets-extra", "VStack")
    }
}

/// A wrapper widget that expands its child to fill available space.
///
/// Similar to Flutter's `Expanded`, this widget forces its child to
/// take up all available space in the parent's flex direction.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::flex::{HStack, Expanded};
/// use nptk_widgets::text::Text;
///
/// HStack::new()
///     .with_child(Text::new("Left"))
///     .with_child(Expanded::new(Text::new("Expands")))  // Takes remaining space
/// ```
pub struct Expanded {
    child: BoxedWidget,
    flex: f32,
    priority: f32,
}

impl Expanded {
    /// Create a new expanded wrapper around a child widget.
    pub fn new(child: impl Widget + Send + Sync + 'static) -> Self {
        Self {
            child: Box::new(child),
            flex: 1.0,
            priority: 0.0,
        }
    }

    /// Set the flex factor for this expanded widget.
    ///
    /// Higher values mean this widget will take more space relative
    /// to other expanded widgets. Default is 1.0.
    pub fn with_flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    /// Set the layout priority for this expanded widget.
    ///
    /// Higher priority widgets get space first and shrink less.
    /// Priority affects flex_grow and flex_shrink calculations.
    /// Default is 0.0 (no priority adjustment).
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }
}

#[async_trait(?Send)]
impl Widget for Expanded {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if !layout.children.is_empty() {
            self.child.render(graphics, &layout.children[0], info, context);
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        let mut child_style = self.child.layout_style(context);
        child_style.style.flex_grow = self.flex;
        child_style.style.flex_shrink = 1.0;
        child_style
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        if !layout.children.is_empty() {
            self.child.update(&layout.children[0], context, info).await
        } else {
            Update::empty()
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets-extra", "Expanded")
    }
}

/// A wrapper widget that allows its child to flex within available space.
///
/// Similar to Flutter's `Flexible`, this widget allows its child to
/// expand or shrink within the parent's flex direction, but doesn't
/// force it to fill all space.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::flex::{HStack, Flexible};
/// use nptk_widgets::text::Text;
///
/// HStack::new()
///     .with_child(Text::new("Left"))
///     .with_child(Flexible::new(Text::new("Can expand")))  // Can grow but not required
/// ```
pub struct Flexible {
    child: BoxedWidget,
    flex: f32,
    priority: f32,
}

impl Flexible {
    /// Create a new flexible wrapper around a child widget.
    pub fn new(child: impl Widget + Send + Sync + 'static) -> Self {
        Self {
            child: Box::new(child),
            flex: 1.0,
            priority: 0.0,
        }
    }

    /// Set the flex factor for this flexible widget.
    pub fn with_flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    /// Set the layout priority for this flexible widget.
    ///
    /// Higher priority widgets get space first and shrink less.
    /// Priority affects flex_grow and flex_shrink calculations.
    /// Default is 0.0 (no priority adjustment).
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }
}

#[async_trait(?Send)]
impl Widget for Flexible {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if !layout.children.is_empty() {
            self.child.render(graphics, &layout.children[0], info, context);
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        let mut child_style = self.child.layout_style(context);
        child_style.style.flex_grow = self.flex;
        child_style.style.flex_shrink = 1.0;
        child_style.style.layout_priority = self.priority;
        child_style
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        if !layout.children.is_empty() {
            self.child.update(&layout.children[0], context, info).await
        } else {
            Update::empty()
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets-extra", "Flexible")
    }
}
