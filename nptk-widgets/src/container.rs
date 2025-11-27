use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Display, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetChildrenExt, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

/// A container widget that can display and layout multiple child widgets.
///
/// The layout of the children (row, column, etc.) depends on the [LayoutStyle] of the container.
///
/// ### Theming
/// The container widget doesn't actually draw anything but the child widgets, so theming is useless.
#[derive(Default)]
pub struct Container {
    style: MaybeSignal<LayoutStyle>,
    children: Vec<BoxedWidget>,
}

impl Container {
    /// Creates a new container with given children.
    pub fn new(children: Vec<BoxedWidget>) -> Self {
        Self {
            style: LayoutStyle::default().into(),
            children: children.into_iter().collect(),
        }
    }
}

impl WidgetChildrenExt for Container {
    fn set_children(&mut self, children: Vec<BoxedWidget>) {
        self.children = children.into_iter().collect();
    }

    fn add_child(&mut self, child: impl Widget + 'static) {
        self.children.push(Box::new(child));
    }
}

impl WidgetLayoutExt for Container {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.style = layout_style.into();
    }
}

impl Widget for Container {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Track which layout child index we're on (skipping Display::None children)
        // The layout_node.children should match the visible children in the same order
        let mut layout_index = 0;
        for child in &mut self.children {
            let child_style = child.layout_style();
            // Skip children with Display::None - they're not in the layout tree
            if child_style.style.display == Display::None {
                continue;
            }
            if layout_index < layout_node.children.len() {
                child.render(
                    graphics,
                    theme,
                    &layout_node.children[layout_index],
                    info,
                    context.clone(),
                );
                layout_index += 1;
            } else {
                // Layout node has fewer children than expected - this shouldn't happen
                // but log it for debugging
                log::warn!(
                    "Container render: layout_node has {} children but expected more (child at index {} is visible but missing from layout)",
                    layout_node.children.len(),
                    layout_index
                );
                break;
            }
        }
        
        // If we didn't render all layout children, log it
        if layout_index < layout_node.children.len() {
            log::warn!(
                "Container render: layout_node has {} children but only rendered {} (some visible children may be missing)",
                layout_node.children.len(),
                layout_index
            );
        }
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Call render_postfix on all children after they've all been rendered
        // This ensures overlays appear on top of all sibling content
        // Track which layout child index we're on (skipping Display::None children)
        let mut layout_index = 0;
        for child in &mut self.children {
            let child_style = child.layout_style();
            // Skip children with Display::None - they're not in the layout tree
            if child_style.style.display == Display::None {
                continue;
            }
            if layout_index < layout_node.children.len() {
                child.render_postfix(
                    graphics,
                    theme,
                    &layout_node.children[layout_index],
                    info,
                    context.clone(),
                );
                layout_index += 1;
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        let style = self.style.get().clone();

        // Include ALL children in the style (don't filter Display::None here)
        // The filtering happens when building the Taffy tree - this ensures
        // the style always matches the widget structure
        let mut children = Vec::with_capacity(self.children.len());
        for child in &self.children {
            children.push(child.layout_style());
        }

        StyleNode { style, children }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Track which layout child index we're on (skipping Display::None children)
        let mut layout_index = 0;
        for child in &mut self.children {
            let child_style = child.layout_style();
            // Skip children with Display::None - they're not in the layout tree
            // But still call update on them so they can detect visibility changes
            if child_style.style.display == Display::None {
                // Create a dummy layout node for hidden children (they won't be rendered)
                let dummy_layout = LayoutNode {
                    layout: Default::default(),
                    children: vec![],
                };
                update.insert(child.update(&dummy_layout, context.clone(), info));
                continue;
            }
            if layout_index < layout.children.len() {
                update.insert(child.update(&layout.children[layout_index], context.clone(), info));
                layout_index += 1;
            }
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Container")
    }
}
