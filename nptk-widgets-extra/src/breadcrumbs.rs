// SPDX-License-Identifier: LGPL-3.0-only
//! Breadcrumbs widget for navigation within hierarchical structures.
//!
//! The breadcrumbs widget provides users with an easy way to navigate within
//! folder structures, web pages, or documentation, and trace their way back
//! to parent levels.

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode, Dimension, FlexDirection};
use nptk_core::signal::MaybeSignal;
use nptk_core::text_render::TextRenderContext;
use nptk_core::vgi::Graphics;
use nptk_core::vg::peniko::Color;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::properties::ThemeProperty;
use nptk_theme::theme::Theme;
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

/// A breadcrumbs navigation widget that displays items horizontally with separators
///
/// ### Theming
/// The breadcrumbs widget supports the following theme properties:
/// - `ColorText` - Text color for breadcrumb items
/// - Custom theme properties for hover and current states
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
///     .with_on_click(|item| {
///         println!("Navigate to: {}", item.id.unwrap_or(item.label));
///         Update::empty()
///     });
/// ```
pub struct Breadcrumbs {
    widget_id: WidgetId,
    items: Vec<BreadcrumbItem>,
    separator: String,
    max_items: Option<usize>,
    show_root: bool,
    show_home_icon: bool,
    font_size: f32,
    spacing: f32,
    layout_style: MaybeSignal<LayoutStyle>,
    on_click: Option<Arc<dyn Fn(&BreadcrumbItem) -> Update + Send + Sync>>,
    hovered_index: Option<usize>,
    item_positions: Vec<(f32, f32, usize)>, // (x, width, original_index) for each visible item
    text_ctx: TextRenderContext,
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
            show_home_icon: false,
            font_size: 14.0,
            spacing: 8.0,
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            on_click: None,
            hovered_index: None,
            item_positions: Vec::new(),
            text_ctx: TextRenderContext::new(),
        }
    }

    /// Set the breadcrumb items
    pub fn with_items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self
    }

    /// Add a single breadcrumb item
    pub fn with_item(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the separator between breadcrumb items (default: " > ")
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Set the maximum number of visible items (older items will be collapsed)
    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items);
        self
    }

    /// Set whether to show the root item when items are collapsed
    pub fn with_show_root(mut self, show_root: bool) -> Self {
        self.show_root = show_root;
        self
    }

    /// Set whether to show a home icon for the first item
    pub fn with_home_icon(mut self, show: bool) -> Self {
        self.show_home_icon = show;
        self
    }

    /// Set the font size for breadcrumb items
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the spacing between items
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
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
    }

    /// Remove the last breadcrumb item (navigate back)
    pub fn pop_item(&mut self) -> Option<BreadcrumbItem> {
        self.items.pop()
    }

    /// Clear all breadcrumb items
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Navigate to a specific breadcrumb by index (removes items after it)
    pub fn navigate_to_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.truncate(index + 1);
        }
    }

    /// Get the visible items considering max_items constraint
    fn get_visible_items(&self) -> Vec<(usize, bool)> {
        // Returns (original_index, is_ellipsis)
        if let Some(max_items) = self.max_items {
            if self.items.len() > max_items {
                let mut visible = Vec::new();
                
                if self.show_root && !self.items.is_empty() {
                    visible.push((0, false)); // Root item
                    
                    if max_items > 2 {
                        visible.push((usize::MAX, true)); // Ellipsis placeholder (using MAX as sentinel)
                        let start_idx = self.items.len().saturating_sub(max_items - 2);
                        for i in start_idx..self.items.len() {
                            visible.push((i, false));
                        }
                    } else if max_items == 2 {
                        visible.push((usize::MAX, true)); // Ellipsis
                        if !self.items.is_empty() {
                            visible.push((self.items.len() - 1, false)); // Last item
                        }
                    }
                } else {
                    if max_items > 1 {
                        visible.push((usize::MAX, true)); // Ellipsis (using MAX as sentinel)
                        let start_idx = self.items.len().saturating_sub(max_items - 1);
                        for i in start_idx..self.items.len() {
                            visible.push((i, false));
                        }
                    } else {
                        if !self.items.is_empty() {
                            visible.push((self.items.len() - 1, false)); // Last item only
                        }
                    }
                }
                
                visible
            } else {
                (0..self.items.len()).map(|i| (i, false)).collect()
            }
        } else {
            (0..self.items.len()).map(|i| (i, false)).collect()
        }
    }

    /// Find which breadcrumb item is at the given position
    fn find_item_at_position(&self, layout: &LayoutNode, x: f32, y: f32) -> Option<usize> {
        // First check if the click is within the widget bounds
        let layout_x = layout.layout.location.x;
        let layout_y = layout.layout.location.y;
        let layout_width = layout.layout.size.width;
        let layout_height = layout.layout.size.height;
        
        if x < layout_x || x > layout_x + layout_width 
            || y < layout_y || y > layout_y + layout_height {
            return None;
        }
        
        // Now check which item the x coordinate falls within
        // Items are stored in render order, so we check in reverse to prioritize later (rightmost) items
        // This handles cases where items might overlap slightly
        for &(item_x, item_width, original_index) in self.item_positions.iter().rev() {
            if x >= item_x && x <= item_x + item_width {
                return Some(original_index);
            }
        }
        None
    }

    /// Get color for a breadcrumb item based on its state
    fn get_item_color(&self, theme: &mut dyn Theme, _index: usize, is_current: bool, is_hovered: bool) -> Color {
        let widget_id = self.widget_id.clone();
        if is_current {
            // Current (last) item - use ColorText (same as normal, but could be styled differently)
            theme
                .get_property(widget_id.clone(), &ThemeProperty::ColorText)
                .or_else(|| theme.get_default_property(&ThemeProperty::ColorText))
                .unwrap_or_else(|| Color::from_rgb8(211, 218, 227)) // Fallback to theme text color
        } else if is_hovered {
            // Hovered item - use ColorHovered if available, otherwise ColorText
            theme
                .get_property(widget_id.clone(), &ThemeProperty::ColorHovered)
                .or_else(|| {
                    theme
                        .get_property(widget_id, &ThemeProperty::ColorText)
                        .or_else(|| theme.get_default_property(&ThemeProperty::ColorText))
                })
                .unwrap_or_else(|| Color::from_rgb8(211, 218, 227))
        } else {
            // Normal clickable item - use ColorText
            theme
                .get_property(widget_id, &ThemeProperty::ColorText)
                .or_else(|| theme.get_default_property(&ThemeProperty::ColorText))
                .unwrap_or_else(|| Color::from_rgb8(211, 218, 227))
        }
    }

    /// Get separator color - use muted text color
    fn get_separator_color(&self, theme: &mut dyn Theme) -> Color {
        let widget_id = self.widget_id.clone();
        // Try to get a border or muted color, fallback to text with reduced alpha
        theme
            .get_property(widget_id.clone(), &ThemeProperty::Border)
            .or_else(|| {
                theme
                    .get_property(widget_id, &ThemeProperty::ColorText)
                    .or_else(|| theme.get_default_property(&ThemeProperty::ColorText))
            })
            .unwrap_or_else(|| Color::from_rgb8(150, 150, 150))
            .with_alpha(0.6)
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
        _: AppContext,
    ) {
        if self.items.is_empty() {
            return;
        }

        let visible_items = self.get_visible_items();
        if visible_items.is_empty() {
            return;
        }

        let base_x = layout.layout.location.x as f64;
        let base_y = layout.layout.location.y as f64;
        let font_size = self.font_size as f64;
        let spacing = self.spacing as f64;
        let last_index = self.items.len() - 1;
        
        // Use base_y directly - Parley's render_text handles baseline positioning internally
        // Similar to how the Text widget renders text
        let text_y = base_y;

        // Clear item positions for accurate click detection
        self.item_positions.clear();

        let mut current_x = base_x;
        let separator_color = self.get_separator_color(theme);

        // Render home icon if enabled and first item is "Home"
        if self.show_home_icon && !visible_items.is_empty() {
            let (first_idx, is_ellipsis) = visible_items[0];
            if !is_ellipsis && first_idx != usize::MAX && first_idx < self.items.len() && self.items[first_idx].label.to_lowercase() == "home" {
                // Draw a simple house icon (square with triangle roof)
                let icon_size = font_size * 0.8;
                
                // Simple house shape using lines/path (simplified - could use SVG icon)
                // For now, just render a placeholder or skip
                // TODO: Add proper home icon rendering
                
                current_x += icon_size + spacing;
            }
        }

        // Render breadcrumb items
        for (vis_idx, (orig_idx, is_ellipsis)) in visible_items.iter().enumerate() {
            if vis_idx > 0 {
                // Render separator (don't render separator before ellipsis, ellipsis acts as separator)
                if !*is_ellipsis {
                    let sep_text = &self.separator;
                    // Measure separator width accurately
                    let sep_width = self.text_ctx.measure_text_width(
                        &mut info.font_context,
                        sep_text,
                        None, // No specific font
                        font_size as f32,
                    ) as f64;
                    
                    let sep_transform = nptk_core::vg::kurbo::Affine::translate((
                        current_x,
                        text_y,
                    ));
                    
                    self.text_ctx.render_text(
                        &mut info.font_context,
                        graphics,
                        sep_text,
                        None, // No specific font
                        font_size as f32,
                        nptk_core::vg::peniko::Brush::Solid(separator_color),
                        sep_transform,
                        true, // hinting
                        None, // No max width
                    );
                    
                    current_x += sep_width + spacing;
                }
            }

            if !is_ellipsis && *orig_idx != usize::MAX && *orig_idx < self.items.len() {
                let item = &self.items[*orig_idx];
                let is_current = *orig_idx == last_index;
                let is_hovered = self.hovered_index == Some(*orig_idx);
                let item_color = self.get_item_color(theme, *orig_idx, is_current, is_hovered);

                // Measure text width for click detection
                let text = if *orig_idx == 0 && self.show_home_icon && item.label.to_lowercase() == "home" {
                    // If home icon is shown, we might skip the "Home" text or render it after icon
                    &item.label
                } else {
                    &item.label
                };

                // Measure text width accurately using TextRenderContext
                let text_width = self.text_ctx.measure_text_width(
                    &mut info.font_context,
                    text,
                    None, // No specific font
                    font_size as f32,
                ) as f64;

                // Store position for click detection (x, width, original_index)
                self.item_positions.push((current_x as f32, text_width as f32, *orig_idx));

                // Render text
                let text_transform = nptk_core::vg::kurbo::Affine::translate((
                    current_x,
                    text_y,
                ));
                
                self.text_ctx.render_text(
                    &mut info.font_context,
                    graphics,
                    text,
                    None, // No specific font
                    font_size as f32,
                    nptk_core::vg::peniko::Brush::Solid(item_color),
                    text_transform,
                    true, // hinting
                    None, // No max width
                );

                current_x += text_width + spacing;
            } else {
                // Render ellipsis
                let ellipsis_text = "...";
                // Measure ellipsis width accurately
                let ellipsis_width = self.text_ctx.measure_text_width(
                    &mut info.font_context,
                    ellipsis_text,
                    None, // No specific font
                    font_size as f32,
                ) as f64;
                
                let ellipsis_transform = nptk_core::vg::kurbo::Affine::translate((
                    current_x,
                    text_y,
                ));
                
                self.text_ctx.render_text(
                    &mut info.font_context,
                    graphics,
                    ellipsis_text,
                    None, // No specific font
                    font_size as f32,
                    nptk_core::vg::peniko::Brush::Solid(separator_color),
                    ellipsis_transform,
                    true, // hinting
                    None, // No max width
                );

                current_x += ellipsis_width + spacing;
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: {
                let mut style = self.layout_style.get().clone();
                // Ensure horizontal layout
                style.flex_direction = FlexDirection::Row;
                // Calculate proper height: use line height calculation (font_size * 1.2) plus padding
                let line_height = self.font_size * 1.2;
                style.size = nalgebra::Vector2::new(
                    Dimension::percent(1.0),
                    Dimension::length(line_height + 4.0), // Height with padding for visual spacing
                );
                style
            },
            children: vec![],
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Handle mouse hover for visual feedback
        if let Some(cursor_pos) = info.cursor_pos {
            let new_hovered = self.find_item_at_position(
                layout,
                cursor_pos.x as f32,
                cursor_pos.y as f32,
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

        update
    }

    fn widget_id(&self) -> WidgetId {
        self.widget_id.clone()
    }
}
