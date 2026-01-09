// SPDX-License-Identifier: LGPL-3.0-only
//! Breadcrumbs widget for navigation within hierarchical structures.
//!
//! The breadcrumbs widget provides users with an easy way to navigate within
//! folder structures, web pages, or documentation, and trace their way back
//! to parent levels.

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode, LengthPercentageAuto};
use nptk_core::signal::MaybeSignal;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nptk_widgets::container::Container;
use nptk_widgets::text::Text;
use std::sync::Arc;

/// Represents a single breadcrumb item in the navigation path
#[derive(Clone, Debug)]
pub struct BreadcrumbItem {
    /// The display text for this breadcrumb
    pub label: String,
    /// Optional identifier for this breadcrumb (e.g., path, URL, ID)
    pub id: Option<String>,
    /// Whether this breadcrumb is clickable
    pub clickable: bool,
}

impl BreadcrumbItem {
    /// Create a new breadcrumb item
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            id: None,
            clickable: true,
        }
    }

    /// Set the identifier for this breadcrumb
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set whether this breadcrumb is clickable
    pub fn with_clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }
}

/// A breadcrumbs navigation widget
///
/// ### Theming
/// The breadcrumbs widget supports the following theme properties:
/// - `breadcrumb_text` - Text color for breadcrumb items
/// - `breadcrumb_text_hover` - Text color when hovering over clickable items
/// - `breadcrumb_text_current` - Text color for the current (last) item
/// - `breadcrumb_separator` - Color for the separator between items
/// - `breadcrumb_background` - Background color for the breadcrumbs container
/// - `breadcrumb_spacing` - Spacing between breadcrumb items and separators
///
/// ### Usage Examples
///
/// ```rust
/// use nptk_widgets_extra::breadcrumbs::{Breadcrumbs, BreadcrumbItem};
///
/// // File system navigation
/// let breadcrumbs = Breadcrumbs::new()
///     .with_items(vec![
///         BreadcrumbItem::new("Home").with_id("/home/user"),
///         BreadcrumbItem::new("Documents").with_id("/home/user/Documents"),
///         BreadcrumbItem::new("Projects").with_id("/home/user/Documents/Projects"),
///         BreadcrumbItem::new("MyApp").with_clickable(false), // Current location
///     ])
///     .with_separator(" > ")
///     .with_on_click(|item| {
///         println!("Navigate to: {}", item.id.unwrap_or(item.label));
///         Update::empty()
///     });
///
/// // Web navigation
/// let web_breadcrumbs = Breadcrumbs::new()
///     .with_items(vec![
///         BreadcrumbItem::new("Documentation").with_id("/docs"),
///         BreadcrumbItem::new("API Reference").with_id("/docs/api"),
///         BreadcrumbItem::new("Widgets").with_id("/docs/api/widgets"),
///         BreadcrumbItem::new("Breadcrumbs").with_clickable(false),
///     ])
///     .with_separator(" / ")
///     .with_max_items(4); // Limit visible items
/// ```
pub struct Breadcrumbs {
    widget_id: WidgetId,
    items: Vec<BreadcrumbItem>,
    separator: String,
    max_items: Option<usize>,
    show_root: bool,
    container: Container,
    layout_style: MaybeSignal<LayoutStyle>,
    on_click: Option<Arc<dyn Fn(&BreadcrumbItem) -> Update + Send + Sync>>,
    hovered_index: Option<usize>,
}

impl std::fmt::Debug for Breadcrumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Breadcrumbs")
            .field("widget_id", &self.widget_id)
            .field("items", &self.items)
            .field("separator", &self.separator)
            .field("max_items", &self.max_items)
            .field("show_root", &self.show_root)
            .field("hovered_index", &self.hovered_index)
            .finish()
    }
}

impl Breadcrumbs {
    /// Create a new breadcrumbs widget
    pub fn new() -> Self {
        Self {
            widget_id: WidgetId::new("nptk_widgets_extra", "Breadcrumbs"),
            items: Vec::new(),
            separator: " > ".to_string(),
            max_items: None,
            show_root: true,
            container: Container::new_empty(),
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            on_click: None,
            hovered_index: None,
        }
    }

    /// Set the breadcrumb items
    pub fn with_items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self.rebuild_container();
        self
    }

    /// Add a single breadcrumb item
    pub fn with_item(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self.rebuild_container();
        self
    }

    /// Set the separator between breadcrumb items
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self.rebuild_container();
        self
    }

    /// Set the maximum number of visible items (older items will be collapsed)
    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items);
        self.rebuild_container();
        self
    }

    /// Set whether to show the root item when items are collapsed
    pub fn with_show_root(mut self, show_root: bool) -> Self {
        self.show_root = show_root;
        self.rebuild_container();
        self
    }

    /// Set the callback for when a breadcrumb item is clicked
    pub fn with_on_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(&BreadcrumbItem) -> Update + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(callback));
        self
    }

    /// Get the current breadcrumb items
    pub fn items(&self) -> &[BreadcrumbItem] {
        &self.items
    }

    /// Add a breadcrumb item
    pub fn add_item(&mut self, item: BreadcrumbItem) {
        self.items.push(item);
        self.rebuild_container();
    }

    /// Remove the last breadcrumb item (navigate back)
    pub fn pop_item(&mut self) -> Option<BreadcrumbItem> {
        let item = self.items.pop();
        if item.is_some() {
            self.rebuild_container();
        }
        item
    }

    /// Clear all breadcrumb items
    pub fn clear(&mut self) {
        self.items.clear();
        self.rebuild_container();
    }

    /// Navigate to a specific breadcrumb by index (removes items after it)
    pub fn navigate_to_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.truncate(index + 1);
            self.rebuild_container();
        }
    }

    /// Rebuild the internal container with current items
    fn rebuild_container(&mut self) {
        use nptk_core::layout::{Dimension, FlexDirection, LengthPercentage};

        let mut children = Vec::new();
        let visible_items = self.get_visible_items();

        for (i, (item, is_ellipsis)) in visible_items.iter().enumerate() {
            // Add separator before item (except for first item)
            if i > 0 {
                let separator_text = Text::new(self.separator.clone())
                    .with_layout_style(LayoutStyle {
                        margin: nptk_core::layout::Rect {
                            left: LengthPercentageAuto::length(4.0),
                            right: LengthPercentageAuto::length(4.0),
                            top: LengthPercentageAuto::length(0.0),
                            bottom: LengthPercentageAuto::length(0.0),
                        },
                        ..Default::default()
                    });
                children.push(Box::new(separator_text) as Box<dyn Widget>);
            }

            // Create the breadcrumb item text
            let text_content = if *is_ellipsis {
                "...".to_string()
            } else {
                item.label.clone()
            };

            let item_text = Text::new(text_content)
                .with_layout_style(LayoutStyle {
                    margin: nptk_core::layout::Rect {
                        left: LengthPercentageAuto::length(2.0),
                        right: LengthPercentageAuto::length(2.0),
                        top: LengthPercentageAuto::length(0.0),
                        bottom: LengthPercentageAuto::length(0.0),
                    },
                    ..Default::default()
                });

            children.push(Box::new(item_text) as Box<dyn Widget>);
        }

        // Create horizontal container for breadcrumbs
        self.container = Container::new(children)
            .with_layout_style(LayoutStyle {
                flex_direction: FlexDirection::Row,
                size: nalgebra::Vector2::new(Dimension::auto(), Dimension::auto()),
                padding: nptk_core::layout::Rect {
                    left: LengthPercentage::length(8.0),
                    right: LengthPercentage::length(8.0),
                    top: LengthPercentage::length(4.0),
                    bottom: LengthPercentage::length(4.0),
                },
                ..Default::default()
            });
    }

    /// Get the visible items considering max_items constraint
    fn get_visible_items(&self) -> Vec<(&BreadcrumbItem, bool)> {
        if let Some(max_items) = self.max_items {
            if self.items.len() > max_items {
                let mut visible = Vec::new();
                
                if self.show_root && !self.items.is_empty() {
                    // Show root item
                    visible.push((&self.items[0], false));
                    
                    if max_items > 2 {
                        // Add ellipsis
                        visible.push((&self.items[0], true)); // Dummy item for ellipsis
                        
                        // Show last (max_items - 2) items
                        let start_idx = self.items.len().saturating_sub(max_items - 2);
                        for item in &self.items[start_idx..] {
                            visible.push((item, false));
                        }
                    } else if max_items == 2 {
                        // Show root and last item only
                        visible.push((&self.items[0], true)); // Ellipsis
                        if let Some(last) = self.items.last() {
                            visible.push((last, false));
                        }
                    }
                } else {
                    // No root, show ellipsis and last items
                    if max_items > 1 {
                        visible.push((&self.items[0], true)); // Ellipsis
                        let start_idx = self.items.len().saturating_sub(max_items - 1);
                        for item in &self.items[start_idx..] {
                            visible.push((item, false));
                        }
                    } else {
                        // Show only the last item
                        if let Some(last) = self.items.last() {
                            visible.push((last, false));
                        }
                    }
                }
                
                visible
            } else {
                self.items.iter().map(|item| (item, false)).collect()
            }
        } else {
            self.items.iter().map(|item| (item, false)).collect()
        }
    }

    /// Find which breadcrumb item is at the given position
    fn find_item_at_position(&self, layout: &LayoutNode, x: f32, y: f32) -> Option<usize> {
        let visible_items = self.get_visible_items();
        
        // Check if click is within the breadcrumbs area
        if x < layout.layout.location.x 
            || x > layout.layout.location.x + layout.layout.size.width
            || y < layout.layout.location.y
            || y > layout.layout.location.y + layout.layout.size.height {
            return None;
        }

        // Simple approximation: divide the width by number of visible items
        // In a real implementation, you'd want to track individual item positions
        let item_width = layout.layout.size.width / visible_items.len() as f32;
        let relative_x = x - layout.layout.location.x;
        let item_index = (relative_x / item_width) as usize;
        
        if item_index < visible_items.len() {
            // Map back to original item index
            let (_, is_ellipsis) = visible_items[item_index];
            if !is_ellipsis {
                // Find the actual item index in the original items
                let visible_item = visible_items[item_index].0;
                for (i, item) in self.items.iter().enumerate() {
                    if std::ptr::eq(item, visible_item) {
                        return Some(i);
                    }
                }
            }
        }
        
        None
    }
}

impl Default for Breadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetLayoutExt for Breadcrumbs {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for Breadcrumbs {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Render the container with all breadcrumb items
        self.container.render(graphics, theme, layout, info, context);
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.container.layout_style()],
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Handle mouse hover for visual feedback
        if let Some(cursor_pos) = info.cursor_pos {
            let new_hovered = self.find_item_at_position(
                layout, 
                cursor_pos.x as f32, 
                cursor_pos.y as f32
            );
            
            if new_hovered != self.hovered_index {
                self.hovered_index = new_hovered;
                update |= Update::DRAW;
            }
        } else if self.hovered_index.is_some() {
            self.hovered_index = None;
            update |= Update::DRAW;
        }

        // Handle clicks on breadcrumb items
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Released {
                if let Some(cursor_pos) = info.cursor_pos {
                    if let Some(item_index) = self.find_item_at_position(
                        layout,
                        cursor_pos.x as f32,
                        cursor_pos.y as f32,
                    ) {
                        if item_index < self.items.len() {
                            let item = &self.items[item_index];
                            if item.clickable {
                                // Execute callback if provided
                                if let Some(ref callback) = self.on_click {
                                    update |= callback(item);
                                }
                                
                                // Navigate to this item (remove items after it)
                                self.navigate_to_index(item_index);
                                update |= Update::DRAW;
                            }
                        }
                    }
                }
            }
        }

        // Update the container
        if !layout.children.is_empty() {
            update |= self.container.update(&layout.children[0], context, info);
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        self.widget_id.clone()
    }
}