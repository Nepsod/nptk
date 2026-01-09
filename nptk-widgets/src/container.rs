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
    /// Cache for visible child indices to avoid repeated Display::None checks
    visible_children_cache: Option<Vec<usize>>,
}

impl Container {
    /// Creates a new container with given children.
    pub fn new(children: Vec<BoxedWidget>) -> Self {
        Self {
            style: LayoutStyle::default().into(),
            children: children.into_iter().collect(),
            visible_children_cache: None,
        }
    }

    /// Creates an empty container (useful for builder-style assembly).
    pub fn new_empty() -> Self {
        Self {
            style: LayoutStyle::default().into(),
            children: Vec::new(),
            visible_children_cache: None,
        }
    }

    /// Adds a child and returns self for chaining.
    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self.visible_children_cache = None; // Invalidate cache
        self
    }

    /// Adds multiple children and returns self for chaining.
    pub fn with_children(mut self, children: Vec<BoxedWidget>) -> Self {
        self.children.extend(children);
        self.visible_children_cache = None; // Invalidate cache
        self
    }

    fn for_each_visible_child<F>(&mut self, layout_node: &LayoutNode, mut f: F)
    where
        F: FnMut(&mut BoxedWidget, &LayoutNode, usize),
    {
        let mut layout_index = 0;
        for child in &mut self.children {
            let child_style = child.layout_style();
            if child_style.style.display == Display::None {
                continue;
            }

            if layout_index < layout_node.children.len() {
                f(child, &layout_node.children[layout_index], layout_index);
                layout_index += 1;
            } else {
                log::warn!(
                    "Container: layout has {} children, but child {} is visible without layout entry",
                    layout_node.children.len(),
                    layout_index
                );
                return;
            }
        }

        if layout_index < layout_node.children.len() {
            log::warn!(
                "Container: layout has {} children, but only {} were rendered",
                layout_node.children.len(),
                layout_index
            );
        }
    }

    fn dummy_layout_node() -> LayoutNode {
        LayoutNode {
            layout: Default::default(),
            children: vec![],
        }
    }
}

impl WidgetChildrenExt for Container {
    fn set_children(&mut self, children: Vec<BoxedWidget>) {
        self.children = children.into_iter().collect();
        self.visible_children_cache = None; // Invalidate cache
    }

    fn add_child(&mut self, child: impl Widget + 'static) {
        self.children.push(Box::new(child));
        self.visible_children_cache = None; // Invalidate cache
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
        self.for_each_visible_child(layout_node, |child, child_layout, _idx| {
            child.render(graphics, theme, child_layout, info, context.clone());
        });
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        self.for_each_visible_child(layout_node, |child, child_layout, _idx| {
            child.render_postfix(graphics, theme, child_layout, info, context.clone());
        });
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

        let mut layout_index = 0;
        for child in &mut self.children {
            let child_style = child.layout_style();
            if child_style.style.display == Display::None {
                update.insert(child.update(&Self::dummy_layout_node(), context.clone(), info));
                continue;
            }

            if layout_index < layout.children.len() {
                update.insert(child.update(&layout.children[layout_index], context.clone(), info));
                layout_index += 1;
            } else {
                log::warn!(
                    "Container update: layout has {} children, but child {} is visible without layout entry",
                    layout.children.len(),
                    layout_index
                );
                break;
            }
        }

        if layout_index < layout.children.len() {
            log::warn!(
                "Container update: layout has {} children, but only {} were updated",
                layout.children.len(),
                layout_index
            );
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Container")
    }
}
