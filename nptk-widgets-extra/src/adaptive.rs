// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Breakpoint, Breakpoints, Constraints, LayoutNode, StyleNode, LayoutContext};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, BoxedWidget};
use async_trait::async_trait;

/// A widget that adapts its layout based on breakpoints.
///
/// This widget switches between different child layouts based on the
/// available width, using breakpoints to determine which layout to use.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::adaptive::Adaptive;
/// use nptk_widgets_extra::flex::{HStack, VStack};
///
/// Adaptive::new()
///     .with_small(|| Box::new(VStack::new()) as Box<dyn Widget + Send + Sync>)
///     .with_medium(|| Box::new(HStack::new()) as Box<dyn Widget + Send + Sync>)
///     .with_large(|| Box::new(HStack::new()) as Box<dyn Widget + Send + Sync>)
/// ```
pub struct Adaptive {
    breakpoints: Breakpoints,
    small_builder: Option<Box<dyn Fn() -> BoxedWidget + Send + Sync>>,
    medium_builder: Option<Box<dyn Fn() -> Box<dyn Widget + Send + Sync> + Send + Sync>>,
    large_builder: Option<Box<dyn Fn() -> Box<dyn Widget + Send + Sync> + Send + Sync>>,
    current_widget: Option<BoxedWidget>,
    current_breakpoint: Option<Breakpoint>,
}

impl Adaptive {
    /// Create a new adaptive widget with default breakpoints.
    pub fn new() -> Self {
        Self {
            breakpoints: Breakpoints::default(),
            small_builder: None,
            medium_builder: None,
            large_builder: None,
            current_widget: None,
            current_breakpoint: None,
        }
    }

    /// Set custom breakpoints.
    pub fn with_breakpoints(mut self, breakpoints: Breakpoints) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Set the builder function for small screens.
    pub fn with_small<F>(mut self, builder: F) -> Self
    where
        F: Fn() -> BoxedWidget + Send + Sync + 'static,
    {
        self.small_builder = Some(Box::new(builder));
        self
    }

    /// Set the builder function for medium screens.
    pub fn with_medium<F>(mut self, builder: F) -> Self
    where
        F: Fn() -> BoxedWidget + Send + Sync + 'static,
    {
        self.medium_builder = Some(Box::new(builder));
        self
    }

    /// Set the builder function for large screens.
    pub fn with_large<F>(mut self, builder: F) -> Self
    where
        F: Fn() -> BoxedWidget + Send + Sync + 'static,
    {
        self.large_builder = Some(Box::new(builder));
        self
    }
}

impl Default for Adaptive {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for Adaptive {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if let Some(ref mut widget) = self.current_widget {
            if !layout.children.is_empty() {
                widget.render(graphics, &layout.children[0], info, context);
            }
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: Default::default(),
            children: if let Some(ref widget) = self.current_widget {
                vec![widget.layout_style(context)]
            } else {
                vec![]
            },
            measure_func: None,
        }
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        let width = layout.layout.size.width;
        let breakpoint = self.breakpoints.get_breakpoint(width);

        // Check if breakpoint changed
        let breakpoint_changed = self
            .current_breakpoint
            .map(|bp| bp != breakpoint)
            .unwrap_or(true);

        if breakpoint_changed {
            // Build the appropriate widget for this breakpoint
            let builder = match breakpoint {
                Breakpoint::Small => self.small_builder.as_ref(),
                Breakpoint::Medium => self.medium_builder.as_ref(),
                Breakpoint::Large => self.large_builder.as_ref(),
            };

            if let Some(builder_fn) = builder {
                self.current_widget = Some(builder_fn());
                self.current_breakpoint = Some(breakpoint);
                return Update::LAYOUT | Update::DRAW;
            }
        }

        // Update the current widget
        if let Some(ref mut widget) = self.current_widget {
            if !layout.children.is_empty() {
                widget.update(&layout.children[0], context, info).await
            } else {
                Update::empty()
            }
        } else {
            Update::empty()
        }
    }

}
