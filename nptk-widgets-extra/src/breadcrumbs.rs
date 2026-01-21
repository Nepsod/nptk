// SPDX-License-Identifier: LGPL-3.0-only
//! Breadcrumbs widget for navigation within hierarchical structures.
//!
//! The breadcrumbs widget provides users with an easy way to navigate within
//! folder structures, web pages, or documentation, and trace their way back
//! to parent levels.

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode, Dimension, FlexDirection, Layout, LayoutContext};
use nptk_core::menu::unified::{MenuTemplate, MenuItem as UnifiedMenuItem};
use nptk_core::menu::commands::MenuCommand;
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vgi::Graphics;
use nptk_core::vg::kurbo::Rect;
use nptk_core::vg::peniko::Color;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_core::theme::{ColorRole, Palette};
use crate::menu_popup::MenuPopup;
use std::sync::Arc;
use async_trait::async_trait;

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
/// - `ColorHovered` - Color for hovered items
/// - `Border` - Color for separators between items
///
/// ### Neighbor Items
/// Breadcrumb items can dynamically show neighbor items (e.g., sibling directories at the same level).
/// When clicking on a separator (e.g., " > "), if a `neighbors_provider` callback is set and returns neighbors
/// for the item before the separator, a popup menu will appear showing all available neighbors.
/// This is useful for file managers where each directory level has different siblings that must be fetched on-the-fly.
/// Clicking on the breadcrumb item itself navigates to it normally.
///
/// ### Usage Examples
///
/// ```rust
/// use nptk_widgets_extra::breadcrumbs::{Breadcrumbs, BreadcrumbItem};
///
/// // File system navigation with dynamic neighbors
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
///     })
///     .with_neighbors_provider(|item| {
///         // Dynamically fetch siblings based on the item's ID/path
///         // For example, if item.id is "/home/user", return sibling directories
///         if let Some(id) = &item.id {
///             if id == "/home/user" {
///                 Some(vec![
///                     BreadcrumbItem::new("Documents").with_id("/home/user/Documents"),
///                     BreadcrumbItem::new("Downloads").with_id("/home/user/Downloads"),
///                     BreadcrumbItem::new("Pictures").with_id("/home/user/Pictures"),
///                 ])
///             } else {
///                 None // No neighbors for this item
///             }
///         } else {
///             None
///         }
///     })
///     .with_on_neighbor_select(|original_item, selected_neighbor| {
///         println!("Switching from {} to {}", original_item.label, selected_neighbor.label);
///         Update::empty()
///     });
/// ```
pub struct Breadcrumbs {
    items: MaybeSignal<Vec<BreadcrumbItem>>,
    items_signal: Option<StateSignal<Vec<BreadcrumbItem>>>,
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
    separator_positions: Vec<(f32, f32, usize)>, // (x, width, item_index_before_separator) for each separator
    text_ctx: TextRenderContext,
    /// Currently open neighbor popup menu
    neighbor_popup: Option<MenuPopup>,
    /// Index of the breadcrumb item that opened the popup
    popup_item_index: Option<usize>,
    /// Callback for when a neighbor item is selected
    on_neighbor_select: Option<Arc<dyn Fn(&BreadcrumbItem, &BreadcrumbItem) -> Update + Send + Sync>>,
    /// Callback to dynamically fetch neighbor items for a breadcrumb (e.g., sibling directories)
    /// Returns None if no neighbors are available, or Some(neighbors) if available
    neighbors_provider: Option<Arc<dyn Fn(&BreadcrumbItem) -> Option<Vec<BreadcrumbItem>> + Send + Sync>>,
}

impl std::fmt::Debug for Breadcrumbs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Breadcrumbs")
            .field("items", &self.get_items_vec())
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
            items: MaybeSignal::value(Vec::new()),
            items_signal: None,
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
            separator_positions: Vec::new(),
            text_ctx: TextRenderContext::new(),
            neighbor_popup: None,
            popup_item_index: None,
            on_neighbor_select: None,
            neighbors_provider: None,
        }
    }

    /// Set the breadcrumb items
    pub fn with_items(mut self, items: impl Into<MaybeSignal<Vec<BreadcrumbItem>>>) -> Self {
        self.items = items.into();
        // Reset state when items change
        self.hovered_index = None;
        self.item_positions.clear();
        self.items_signal = None; // Clear signal reference for static items
        self
    }

    /// Set the breadcrumb items using a reactive signal
    /// This allows external code to update breadcrumbs reactively
    pub fn with_items_signal(mut self, items_signal: StateSignal<Vec<BreadcrumbItem>>) -> Self {
        self.items = MaybeSignal::signal(Box::new(items_signal.clone()));
        self.items_signal = Some(items_signal);
        // Reset state when items change
        self.hovered_index = None;
        self.item_positions.clear();
        self
    }

    /// Add a single breadcrumb item
    pub fn with_item(mut self, item: BreadcrumbItem) -> Self {
        if let Some(ref signal) = self.items_signal {
            signal.mutate(|items| items.push(item));
        } else {
            // For fixed signals, we can't modify - this is a no-op
            // Users should use with_items() or with_items_signal() instead
        }
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

    /// Set the callback for when a neighbor item is selected from the popup
    /// The callback receives (original_item, selected_neighbor)
    pub fn with_on_neighbor_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(&BreadcrumbItem, &BreadcrumbItem) -> Update + Send + Sync + 'static,
    {
        self.on_neighbor_select = Some(Arc::new(callback));
        self
    }

    /// Set a callback to dynamically fetch neighbor items for each breadcrumb item.
    /// This is called when clicking on a separator (e.g., " > ") to show neighbors of the item before the separator.
    /// The callback receives the breadcrumb item and returns Some(neighbors) if neighbors are available,
    /// or None if no neighbors should be shown for this item.
    /// 
    /// Clicking on the separator shows the popup menu with neighbors, while clicking on the item itself navigates to it.
    /// 
    /// This is useful for file managers where each directory level has different siblings
    /// that cannot be hardcoded and must be fetched on-the-fly.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use nptk_widgets_extra::breadcrumbs::{Breadcrumbs, BreadcrumbItem};
    /// 
    /// Breadcrumbs::new()
    ///     .with_neighbors_provider(|item| {
    ///         // Query filesystem for sibling directories
    ///         // For example, if item.id is "/home/user/Documents", 
    ///         // return siblings like ["Downloads", "Pictures", "Videos"]
    ///         Some(vec![
    ///             BreadcrumbItem::new("Downloads"),
    ///             BreadcrumbItem::new("Pictures"),
    ///             BreadcrumbItem::new("Videos"),
    ///         ])
    ///     });
    /// ```
    pub fn with_neighbors_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(&BreadcrumbItem) -> Option<Vec<BreadcrumbItem>> + Send + Sync + 'static,
    {
        self.neighbors_provider = Some(Arc::new(provider));
        self
    }

    /// Get the current breadcrumb items
    pub fn items(&self) -> Vec<BreadcrumbItem> {
        self.items.get().iter().cloned().collect()
    }

    /// Get the items signal for observability (returns None if items are static)
    pub fn get_items_signal(&self) -> Option<StateSignal<Vec<BreadcrumbItem>>> {
        self.items_signal.clone()
    }

    /// Add a breadcrumb item
    pub fn add_item(&mut self, item: BreadcrumbItem) {
        if let Some(ref signal) = self.items_signal {
            signal.mutate(|items| items.push(item));
        }
        // For fixed signals, this is a no-op - items are immutable
    }

    /// Remove the last breadcrumb item (navigate back)
    pub fn pop_item(&mut self) -> Option<BreadcrumbItem> {
        if let Some(ref signal) = self.items_signal {
            let mut result = None;
            signal.mutate(|items| {
                result = items.pop();
            });
            // Reset state when items change
            self.hovered_index = None;
            self.item_positions.clear();
            self.separator_positions.clear();
            // Close popup if open (item might be invalid now)
            if self.popup_item_index.is_some() {
                self.close_neighbor_popup();
            }
            result
        } else {
            // For fixed signals, this is a no-op
            None
        }
    }

    /// Clear all breadcrumb items
    pub fn clear(&mut self) {
        if let Some(ref signal) = self.items_signal {
            signal.mutate(|items| items.clear());
        }
        // Reset state when items change
        self.hovered_index = None;
        self.item_positions.clear();
        self.separator_positions.clear();
        // Close popup since all items are cleared
        if self.popup_item_index.is_some() {
            self.close_neighbor_popup();
        }
    }

    /// Navigate to a specific breadcrumb by index (removes items after it)
    pub fn navigate_to_index(&mut self, index: usize) {
        if let Some(ref signal) = self.items_signal {
            let items_len = {
                let items_ref = signal.get();
                items_ref.len()
            };
            if index < items_len {
                let old_len = items_len;
                signal.mutate(|items| {
                    items.truncate(index + 1);
                });
                let new_len = {
                    let items_ref = signal.get();
                    items_ref.len()
                };
                // Reset hovered_index if it's now out of bounds or if items changed
                if old_len != new_len {
                    self.hovered_index = None;
                    self.item_positions.clear();
                    self.separator_positions.clear();
                    // Close popup if item index is now out of bounds
                    if let Some(popup_idx) = self.popup_item_index {
                        if popup_idx >= new_len {
                            self.close_neighbor_popup();
                        }
                    }
                } else if let Some(hovered) = self.hovered_index {
                    if hovered >= new_len {
                        self.hovered_index = None;
                        self.item_positions.clear();
                        self.separator_positions.clear();
                    }
                }
                // Also check popup_item_index even if items.len() didn't change
                if let Some(popup_idx) = self.popup_item_index {
                    if popup_idx >= new_len {
                        self.close_neighbor_popup();
                    }
                }
            } else {
                // index >= items_len, close popup if it references an invalid index
                if let Some(popup_idx) = self.popup_item_index {
                    if popup_idx >= items_len {
                        self.close_neighbor_popup();
                    }
                }
            }
        }
        // For fixed signals, this is a no-op - items are immutable
    }

    /// Helper method to get the current items Vec
    fn get_items_vec(&self) -> Vec<BreadcrumbItem> {
        self.items.get().iter().cloned().collect()
    }

    /// Get the visible items considering max_items constraint
    fn get_visible_items(&self) -> Vec<(usize, bool)> {
        let items = self.get_items_vec();
        self.get_visible_items_from_slice(&items)
    }

    /// Get the visible items considering max_items constraint (internal helper)
    fn get_visible_items_from_slice(&self, items: &[BreadcrumbItem]) -> Vec<(usize, bool)> {
        let items_len = items.len();
        // Returns (original_index, is_ellipsis)
        if let Some(max_items) = self.max_items {
            if items_len > max_items {
                let mut visible = Vec::new();
                
                if self.show_root && !items.is_empty() {
                    visible.push((0, false)); // Root item
                    
                    if max_items > 2 {
                        visible.push((usize::MAX, true)); // Ellipsis placeholder (using MAX as sentinel)
                        let start_idx = items_len.saturating_sub(max_items - 2);
                        // Ensure start_idx > 0 to avoid duplicating root item
                        let start_idx = start_idx.max(1);
                        for i in start_idx..items_len {
                            visible.push((i, false));
                        }
                    } else if max_items == 2 {
                        visible.push((usize::MAX, true)); // Ellipsis
                        // Skip last item if it's the same as root (max_items == 2 means only room for root + last)
                        // But if there are more than 2 items, we want root + ellipsis + last
                        if items_len > 1 {
                            visible.push((items_len - 1, false)); // Last item
                        }
                    } else if max_items == 1 {
                        // Only show root, no ellipsis or last item
                        // visible already contains root, we're done
                    }
                } else {
                    if max_items > 1 {
                        visible.push((usize::MAX, true)); // Ellipsis (using MAX as sentinel)
                        let start_idx = items_len.saturating_sub(max_items - 1);
                        for i in start_idx..items_len {
                            visible.push((i, false));
                        }
                    } else {
                        // max_items == 1, show only last item
                        if !items.is_empty() {
                            visible.push((items_len - 1, false)); // Last item only
                        }
                    }
                }
                
                visible
            } else {
                (0..items_len).map(|i| (i, false)).collect()
            }
        } else {
            (0..items_len).map(|i| (i, false)).collect()
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
        
        // If item_positions is empty, render hasn't happened yet - can't detect clicks accurately
        // This can happen if update() is called before the first render()
        if self.item_positions.is_empty() {
            return None;
        }
        
        let items = self.get_items_vec();
        let items_len = items.len();
        
        // Validate that hovered_index is still valid if set
        if let Some(hovered) = self.hovered_index {
            if hovered >= items_len {
                // This shouldn't happen with proper state management, but guard against it
                return None;
            }
        }
        
        // Now check which item the x coordinate falls within
        // Items are stored in render order, so we check in reverse to prioritize later (rightmost) items
        // This handles cases where items might overlap slightly
        for &(item_x, item_width, original_index) in self.item_positions.iter().rev() {
            // Validate original_index is in bounds before checking bounds
            if original_index < items_len && x >= item_x && x < item_x + item_width {
                return Some(original_index);
            }
        }
        None
    }

    /// Find which separator is at the given position
    /// Returns the index of the item that comes before the separator
    fn find_separator_at_position(&self, layout: &LayoutNode, x: f32, y: f32) -> Option<usize> {
        // First check if the click is within the widget bounds
        let layout_x = layout.layout.location.x;
        let layout_y = layout.layout.location.y;
        let layout_width = layout.layout.size.width;
        let layout_height = layout.layout.size.height;
        
        if x < layout_x || x > layout_x + layout_width 
            || y < layout_y || y > layout_y + layout_height {
            return None;
        }
        
        // If separator_positions is empty, render hasn't happened yet or no separators
        if self.separator_positions.is_empty() {
            return None;
        }
        
        let items_len = self.get_items_vec().len();
        
        // Check which separator the x coordinate falls within
        // Check in reverse order to prioritize later (rightmost) separators
        for &(sep_x, sep_width, item_index_before) in self.separator_positions.iter().rev() {
            // Validate item_index_before is in bounds
            if item_index_before < items_len && x >= sep_x && x < sep_x + sep_width {
                return Some(item_index_before);
            }
        }
        None
    }

    /// Get color for a breadcrumb item based on its state
    fn get_item_color(&self, palette: &Palette, _index: usize, is_current: bool, is_hovered: bool) -> Color {
        if is_current {
            // Current (last) item - use BaseText
            palette.color(ColorRole::BaseText)
        } else if is_hovered {
            // Hovered item - use HoverHighlight
            palette.color(ColorRole::HoverHighlight)
        } else {
            // Normal clickable item - use BaseText
            palette.color(ColorRole::BaseText)
        }
    }

    /// Get separator color - use muted text color
    fn get_separator_color(&self, palette: &Palette) -> Color {
        // Use ThreedShadow1 with reduced alpha for separator
        palette.color(ColorRole::ThreedShadow1).with_alpha(0.6)
    }

    /// Show neighbor popup for the given item index
    fn show_neighbor_popup(&mut self, item_index: usize) {
        let items = self.get_items_vec();
        if item_index >= items.len() {
            return;
        }

        let item = &items[item_index];
        
        // Get neighbors using the provider callback
        let neighbors = if let Some(ref provider) = self.neighbors_provider {
            provider(item)
        } else {
            None
        };

        let neighbors = match neighbors {
            Some(neighbors) if !neighbors.is_empty() => neighbors,
            _ => return, // No neighbors available
        };

        // Convert neighbor items to menu items
        let menu_items: Vec<UnifiedMenuItem> = neighbors
            .iter()
            .enumerate()
            .map(|(idx, neighbor)| {
                let neighbor_clone = neighbor.clone();
                let item_clone = item.clone();
                let on_neighbor_select_clone = self.on_neighbor_select.clone();

                UnifiedMenuItem::new(
                    MenuCommand::Custom(idx as u32),
                    neighbor.label.clone(),
                )
                .with_enabled(true)
                .with_action(move || {
                    // Call the neighbor select callback if provided
                    if let Some(ref callback) = on_neighbor_select_clone {
                        callback(&item_clone, &neighbor_clone);
                    }
                    Update::FORCE // Signal that menu should close
                })
            })
            .collect();

        let template = MenuTemplate::from_items("breadcrumbs_neighbors", menu_items);
        let popup = MenuPopup::new(template);
        self.neighbor_popup = Some(popup);
        self.popup_item_index = Some(item_index);
    }

    /// Close the neighbor popup if open
    fn close_neighbor_popup(&mut self) {
        self.neighbor_popup = None;
        self.popup_item_index = None;
    }

    /// Get the position and bounds for a breadcrumb item at the given index
    fn get_item_bounds(&self, layout: &LayoutNode, item_index: usize) -> Option<(f64, f64, f64, f64)> {
        // Find the position for this item in item_positions
        for &(item_x, item_width, original_index) in &self.item_positions {
            if original_index == item_index {
                let layout_y = layout.layout.location.y as f64;
                let item_height = (self.font_size * 1.5) as f64;
                
                return Some((
                    item_x as f64,
                    layout_y,
                    item_width as f64,
                    item_height,
                ));
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

#[async_trait(?Send)]
impl Widget for Breadcrumbs {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        let palette = context.palette();
        let items = self.get_items_vec();
        if items.is_empty() {
            return;
        }

        let visible_items = self.get_visible_items_from_slice(&items);
        if visible_items.is_empty() {
            return;
        }

        let base_x = layout.layout.location.x as f64;
        let base_y = layout.layout.location.y as f64;
        let font_size = self.font_size as f64;
        let spacing = self.spacing as f64;
        let items_len = items.len();
        let last_index = items_len - 1;
        
        // Use base_y directly - Parley's render_text handles baseline positioning internally
        // Similar to how the Text widget renders text
        let text_y = base_y;

        // Clear item and separator positions for accurate click detection
        self.item_positions.clear();
        self.separator_positions.clear();

        let mut current_x = base_x;
        let separator_color = self.get_separator_color(palette);

        // Render home icon if enabled and first item is "Home"
        if self.show_home_icon && !visible_items.is_empty() {
            let (first_idx, is_ellipsis) = visible_items[0];
            if !is_ellipsis && first_idx != usize::MAX && first_idx < items_len && items[first_idx].label.to_lowercase() == "home" {
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
                    
                    // Store separator position for click detection
                    // The separator is associated with the item that comes before it
                    let prev_orig_idx = if vis_idx > 0 {
                        visible_items[vis_idx - 1].0
                    } else {
                        0
                    };
                    
                    // Only store separator position if the previous item is valid (not ellipsis)
                    if prev_orig_idx != usize::MAX && prev_orig_idx < items_len {
                        self.separator_positions.push((current_x as f32, sep_width as f32, prev_orig_idx));
                    }
                    
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

            if !is_ellipsis && *orig_idx != usize::MAX && *orig_idx < items_len {
                let item = &items[*orig_idx];
                let is_current = *orig_idx == last_index;
                let is_hovered = self.hovered_index == Some(*orig_idx);
                let item_color = self.get_item_color(palette, *orig_idx, is_current, is_hovered);

                // Measure text width for click detection
                let text = &item.label;

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

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
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
            measure_func: None,
        }
    }

    async fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let items = self.get_items_vec();
        let items_len = items.len();

        // Validate hovered_index is still valid (items might have changed)
        if let Some(hovered) = self.hovered_index {
            if hovered >= items_len {
                self.hovered_index = None;
                update |= Update::DRAW;
            }
        }

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

        // Handle neighbor popup updates if open
        if let Some(popup_item_index) = self.popup_item_index {
            // Validate popup_item_index is still valid (items might have changed)
            if popup_item_index >= items_len {
                // Item index is out of bounds - close popup
                self.close_neighbor_popup();
                update |= Update::DRAW;
            } else {
                // Get item bounds before borrowing self mutably
                let item_bounds = self.get_item_bounds(layout, popup_item_index);
                
                if let Some(ref mut popup) = self.neighbor_popup {
                    if let Some((item_x, item_y, _item_width, item_height)) = item_bounds {
                        let (popup_width, popup_height) = popup.calculate_size_with_contexts(&mut self.text_ctx, &mut info.font_context);
                        let popup_x = item_x;
                        let popup_y = item_y + item_height;

                        let mut popup_layout = LayoutNode {
                            layout: Layout::default(),
                            children: Vec::new(),
                        };
                        popup_layout.layout.location.x = popup_x as f32;
                        popup_layout.layout.location.y = popup_y as f32;
                        popup_layout.layout.size.width = popup_width as f32;
                        popup_layout.layout.size.height = popup_height as f32;

                        let popup_update = popup.update(&popup_layout, _context.clone(), info).await;
                        update |= popup_update;

                        // If popup returns FORCE, it means an item was selected - close the popup
                        if popup_update.contains(Update::FORCE) {
                            // Mutable borrow is automatically released when popup goes out of scope
                            self.close_neighbor_popup();
                            update |= Update::DRAW;
                        } else {
                            // Check if clicking outside the popup should close it
                            if let Some(cursor_pos) = info.cursor_pos {
                                let popup_rect = Rect::new(
                                    popup_x,
                                    popup_y,
                                    popup_x + popup_width,
                                    popup_y + popup_height,
                                );
                                
                                // Also check if clicking on breadcrumb items
                                for (_, button, state) in &info.buttons {
                                    if *button == MouseButton::Left && *state == ElementState::Released {
                                        if !popup_rect.contains((cursor_pos.x, cursor_pos.y)) {
                                            // Mutable borrow is automatically released when popup goes out of scope
                                            // Check if click is on a different breadcrumb item
                                            if let Some(clicked_index) = self.find_item_at_position(
                                                layout,
                                                cursor_pos.x as f32,
                                                cursor_pos.y as f32,
                                            ) {
                                                if clicked_index != popup_item_index {
                                                    self.close_neighbor_popup();
                                                    update |= Update::DRAW;
                                                }
                                            } else {
                                                // Click outside both popup and breadcrumbs - close popup
                                                self.close_neighbor_popup();
                                                update |= Update::DRAW;
                                            }
                                            break; // Exit after handling click
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Item is not visible (e.g., due to max_items) - close popup
                        // Mutable borrow is automatically released when popup goes out of scope
                        self.close_neighbor_popup();
                        update |= Update::DRAW;
                    }
                }
            }
        }

        // Handle clicks on breadcrumb items and separators
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Released {
                if let Some(cursor_pos) = info.cursor_pos {
                    // First check if clicking on a separator
                    if let Some(item_index_before_separator) = self.find_separator_at_position(
                        layout,
                        cursor_pos.x as f32,
                        cursor_pos.y as f32,
                    ) {
                        if item_index_before_separator < items_len {
                            let item = &items[item_index_before_separator];
                            
                            // Check if neighbors are available using the provider
                            let has_neighbors = if let Some(ref provider) = self.neighbors_provider {
                                provider(item).map(|n| !n.is_empty()).unwrap_or(false)
                            } else {
                                false
                            };

                            // Show popup if neighbors are available
                            if has_neighbors {
                                // Toggle popup: if already open for this item, close it; otherwise open it
                                if self.popup_item_index == Some(item_index_before_separator) {
                                    // Popup is already open for this separator - close it
                                    self.close_neighbor_popup();
                                } else {
                                    // Close any existing popup first
                                    if self.popup_item_index.is_some() {
                                        self.close_neighbor_popup();
                                    }
                                    // Open popup for this item
                                    self.show_neighbor_popup(item_index_before_separator);
                                }
                                update |= Update::DRAW;
                            }
                            // If no neighbors, clicking on separator does nothing
                        }
                    } else if let Some(item_index) = self.find_item_at_position(
                        layout,
                        cursor_pos.x as f32,
                        cursor_pos.y as f32,
                    ) {
                        // Clicked on an item (not separator) - navigate normally
                        if item_index < items_len {
                            let item = &items[item_index];
                            if item.clickable {
                                // Execute callback if provided
                                if let Some(ref callback) = self.on_click {
                                    update |= callback(item);
                                }
                                
                                // Navigate to this item (remove items after it)
                                // Close any open popups since we're navigating
                                if self.popup_item_index.is_some() {
                                    self.close_neighbor_popup();
                                }
                                // Note: navigate_to_index will reset hovered_index and item_positions if needed
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

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Render neighbor popup if open
        if let Some(popup_item_index) = self.popup_item_index {
            // Validate popup_item_index is still valid and item is visible
            let items_len = self.get_items_vec().len();
            if popup_item_index >= items_len {
                // Item index is out of bounds - popup won't render (will be cleaned up in update)
                return;
            }
            
            // Get item bounds before borrowing self mutably
            let item_bounds = self.get_item_bounds(layout, popup_item_index);
            
            if let Some(ref mut popup) = self.neighbor_popup {
                if let Some((item_x, item_y, _item_width, item_height)) = item_bounds {
                    let (popup_width, popup_height) = popup.calculate_size_with_contexts(&mut self.text_ctx, &mut info.font_context);
                    let popup_x = item_x;
                    let popup_y = item_y + item_height;

                    let mut popup_layout = LayoutNode {
                        layout: Layout::default(),
                        children: Vec::new(),
                    };
                    popup_layout.layout.location.x = popup_x as f32;
                    popup_layout.layout.location.y = popup_y as f32;
                    popup_layout.layout.size.width = popup_width as f32;
                    popup_layout.layout.size.height = popup_height as f32;

                    popup.render(graphics, &popup_layout, info, context);
                }
                // If item_bounds is None, item is not visible (e.g., due to max_items)
                // Popup won't render but will be cleaned up in update()
            }
        }
    }

}
