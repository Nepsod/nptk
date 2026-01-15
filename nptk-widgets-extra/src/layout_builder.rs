// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Constraints, LayoutNode, StyleNode};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, BoxedWidget};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;

/// A widget that provides layout constraints to its child builder function.
///
/// Similar to Flutter's `LayoutBuilder`, this widget allows child widgets
/// to adapt their layout based on the available space from the parent.
///
/// The builder function receives the current constraints and returns a widget
/// that will be rendered. The layout will be recomputed when constraints change
/// (e.g., window resize).
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::layout_builder::LayoutBuilder;
/// use nptk_widgets_extra::flex::{HStack, VStack};
/// use nptk_core::layout::Constraints;
///
/// LayoutBuilder::new(|constraints| {
///     if constraints.max_width > 600.0 {
///         // Wide layout: horizontal
///         Box::new(HStack::new()) as Box<dyn Widget + Send + Sync>
///     } else {
///         // Narrow layout: vertical
///         Box::new(VStack::new()) as Box<dyn Widget + Send + Sync>
///     }
/// })
/// ```
pub struct LayoutBuilder {
    builder: Box<dyn Fn(Constraints) -> BoxedWidget + Send + Sync>,
    current_widget: Option<BoxedWidget>,
    last_constraints: Option<Constraints>,
}

impl LayoutBuilder {
    /// Create a new layout builder with the given builder function.
    ///
    /// The builder function will be called with the current constraints
    /// whenever the layout needs to be recomputed.
    pub fn new<F>(builder: F) -> Self
    where
        F: Fn(Constraints) -> BoxedWidget + Send + Sync + 'static,
    {
        Self {
            builder: Box::new(builder),
            current_widget: None,
            last_constraints: None,
        }
    }
}

#[async_trait(?Send)]
impl Widget for LayoutBuilder {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if let Some(ref mut widget) = self.current_widget {
            if !layout.children.is_empty() {
                widget.render(graphics, theme, &layout.children[0], info, context);
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        // We need constraints to build the child widget, but we don't have them here.
        // The actual widget building happens in update() when we have layout information.
        // For now, return an empty style node - the real child will be built in update().
        StyleNode {
            style: Default::default(),
            children: if let Some(ref widget) = self.current_widget {
                vec![widget.layout_style()]
            } else {
                vec![]
            },
        }
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        // Extract constraints from the layout
        let constraints = Constraints::new(
            Vector2::new(0.0, 0.0), // min (we don't have this info easily)
            Vector2::new(
                layout.layout.size.width.max(0.0),
                layout.layout.size.height.max(0.0),
            ),
        );

        // Check if constraints changed
        let constraints_changed = self
            .last_constraints
            .map(|c| c != constraints)
            .unwrap_or(true);

        if constraints_changed {
            // Rebuild the widget with new constraints
            self.current_widget = Some((self.builder)(constraints));
            self.last_constraints = Some(constraints);
            return Update::LAYOUT | Update::DRAW;
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

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets-extra", "LayoutBuilder")
    }
}
