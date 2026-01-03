// SPDX-License-Identifier: MIT OR Apache-2.0

//! Menu popup widget for dropdown menus
//!
//! This widget is designed to be used as overlay content for menu dropdowns.
//! It provides a clean, themed popup menu that can be positioned anywhere on screen.

mod constants;
mod interaction;
mod layout;
mod rendering;
mod size;
mod theme;
#[cfg(feature = "unified-menu")]
mod conversion;

pub use constants::*;
pub use interaction::*;
pub use layout::*;
pub use rendering::*;
pub use size::*;
pub use theme::*;

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::menu::unified::MenuTemplate;
use nptk_core::menu::render::{render_menu, calculate_menu_size};
use nptk_core::signal::MaybeSignal;
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{Point, Rect};
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nptk_core::vgi::Graphics;
use std::sync::Arc;

/// A popup menu widget that displays a list of menu items
/// 
/// Supports both the legacy MenuBarItem API and the new unified MenuTemplate system.
pub struct MenuPopup {
    /// The menu items to display (legacy API)
    items: Vec<MenuBarItem>,
    /// The menu template to display (new unified API)
    template: Option<MenuTemplate>,
    /// Layout style for the popup
    layout_style: MaybeSignal<LayoutStyle>,
    /// Currently hovered item index
    hovered_index: Option<usize>,
    /// Text rendering context
    text_render_context: TextRenderContext,
    /// Callback to execute when an item is selected
    on_item_selected: Option<Arc<dyn Fn(usize) -> Update + Send + Sync>>,
    /// Callback to execute when the popup should be closed
    on_close: Option<Arc<dyn Fn() -> Update + Send + Sync>>,

    // Submenu support
    /// Currently open child popup
    child_popup: Option<Box<MenuPopup>>,
    /// Index of the item that opened the child popup
    open_item_index: Option<usize>,
}

/// Represents a menu item in the popup menu (reused from menubar)
#[derive(Clone)]
pub struct MenuBarItem {
    /// Unique identifier for the menu item
    pub id: String,
    /// Display text for the menu item
    pub label: String,
    /// Optional keyboard shortcut text (e.g., "Ctrl+N")
    pub shortcut: Option<String>,
    /// Whether the menu item is enabled/clickable
    pub enabled: bool,
    /// Child menu items for submenus
    pub submenu: Vec<MenuBarItem>,
    /// Callback function to execute when the menu item is activated
    pub on_activate: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
}

impl MenuBarItem {
    /// Create a new menu item
    pub fn new(id: impl ToString, label: impl ToString) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            shortcut: None,
            enabled: true,
            submenu: Vec::new(),
            on_activate: None,
        }
    }

    /// Set the keyboard shortcut for this item
    pub fn with_shortcut(mut self, shortcut: impl ToString) -> Self {
        self.shortcut = Some(shortcut.to_string());
        self
    }

    /// Set whether this item is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a submenu item
    pub fn with_submenu_item(mut self, item: MenuBarItem) -> Self {
        self.submenu.push(item);
        self
    }

    /// Set the callback for when this item is activated
    pub fn with_on_activate<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        self.on_activate = Some(Arc::new(callback));
        self
    }

    /// Check if this item has a submenu
    pub fn has_submenu(&self) -> bool {
        !self.submenu.is_empty()
    }
}

impl MenuPopup {
    /// Create a new menu popup
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            layout_style: LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::length(200.0), // Default width
                    Dimension::length(100.0), // Default height
                ),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(0.0),
                    right: LengthPercentage::length(0.0),
                    top: LengthPercentage::length(ITEM_TOP_PADDING as f32),
                    bottom: LengthPercentage::length(ITEM_TOP_PADDING as f32),
                },
                ..Default::default()
            }
            .into(),
            hovered_index: None,
            text_render_context: TextRenderContext::new(),
            on_item_selected: None,
            on_close: None,
            child_popup: None,
            open_item_index: None,
            template: None,
        }
    }

    /// Set the menu items
    pub fn with_items(mut self, items: Vec<MenuBarItem>) -> Self {
        self.items = items;
        self
    }

    /// Set the layout style
    pub fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = layout_style.into();
        self
    }

    /// Set callback for when an item is selected
    pub fn with_on_item_selected<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize) -> Update + Send + Sync + 'static,
    {
        self.on_item_selected = Some(Arc::new(callback));
        self
    }

    /// Set callback for when the popup should be closed
    pub fn with_on_close<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        self.on_close = Some(Arc::new(callback));
        self
    }

    /// Set the menu template (new unified API)
    pub fn with_template(mut self, template: MenuTemplate) -> Self {
        self.template = Some(template);
        self
    }

    /// Calculate the size needed for the popup based on items
    pub fn calculate_size(&self) -> (f64, f64) {
        if let Some(ref template) = self.template {
            // Use unified system size calculation
            // We'll use temporary contexts for measurement
            let mut text_render = TextRenderContext::new();
            // For now, fall back to legacy calculation if template is present but empty
            if template.items.is_empty() {
                size::calculate_popup_size(&self.items)
            } else {
                // Estimate size - proper implementation would need font context
                let height = (template.items.len() as f64 * 24.0) + 8.0;
                let width = 200.0; // Default width
                (width, height)
            }
        } else {
            // Legacy calculation
            size::calculate_popup_size(&self.items)
        }
    }
}

impl Default for MenuPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetLayoutExt for MenuPopup {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for MenuPopup {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "MenuPopup")
    }

    fn layout_style(&self) -> StyleNode {
        let (width, height) = self.calculate_size();
        let mut style = self.layout_style.get().clone();

        // Override size with calculated size
        style.size = nalgebra::Vector2::new(
            Dimension::length(width as f32),
            Dimension::length(height as f32),
        );

        StyleNode {
            style,
            children: Vec::new(),
        }
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Extract theme colors
        let colors = ThemeColors::extract(theme, self.widget_id());

        // Calculate popup size
        let (popup_width, popup_height) = self.calculate_size();

        // Draw popup background
        let popup_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            layout.layout.location.x as f64 + popup_width,
            layout.layout.location.y as f64 + popup_height,
        );

        // Render background and border
        rendering::render_background_and_border(graphics, popup_rect, &colors);

        // Render menu items
        rendering::render_menu_items(
            graphics,
            &mut self.text_render_context,
            &mut info.font_context,
            &self.items,
            popup_rect,
            self.hovered_index,
            &colors,
        );

        // Render child popup if open
        if let Some(ref mut child) = self.child_popup {
            if let Some(open_index) = self.open_item_index {
                let (child_width, child_height) = child.calculate_size();
                let child_layout = rendering::calculate_child_popup_layout_for_render(
                    popup_rect,
                    open_index,
                    child_width,
                    child_height,
                );
                child.render(graphics, theme, &child_layout, info, context);
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Get mouse position
        let cursor_pos = info.cursor_pos.map(|p| nptk_core::vg::kurbo::Point::new(p.x, p.y));

        // Check hover state
        let old_hovered = self.hovered_index;
        let (popup_width, popup_height) = self.calculate_size();
        let popup_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            layout.layout.location.x as f64 + popup_width,
            layout.layout.location.y as f64 + popup_height,
        );

        self.hovered_index = interaction::detect_hovered_item(&self.items, popup_rect, cursor_pos);

        if old_hovered != self.hovered_index {
            update |= Update::DRAW;

            // Handle submenu opening/closing on hover change
            if let Some(hovered) = self.hovered_index {
                if self.open_item_index != Some(hovered) {
                    // Hovered item changed
                    let item = &self.items[hovered];
                    if item.has_submenu() {
                        // Open new submenu
                        self.child_popup = Some(Box::new(interaction::open_submenu(
                            item,
                            self.on_close.clone(),
                        )));
                        self.open_item_index = Some(hovered);
                        update |= Update::DRAW;
                    } else {
                        // Close submenu if hovering over leaf item
                        if self.child_popup.is_some() {
                            self.child_popup = None;
                            self.open_item_index = None;
                            update |= Update::DRAW;
                        }
                    }
                }
            }
        }

        // Update child popup
        let mut child_hovered = false;
        if let Some(ref mut child) = self.child_popup {
            if let Some(open_index) = self.open_item_index {
                let (child_width, child_height) = child.calculate_size();
                let (child_layout, child_rect) = interaction::calculate_child_popup_for_update(
                    popup_rect,
                    open_index,
                    child_width,
                    child_height,
                );
                update |= child.update(&child_layout, context, info);

                // Check if mouse is over child
                child_hovered = interaction::is_child_hovered(child_rect, cursor_pos);
            }
        }

        // Keep parent item selected if child is hovered
        if child_hovered {
            if let Some(open_index) = self.open_item_index {
                if self.hovered_index != Some(open_index) {
                    self.hovered_index = Some(open_index);
                    update |= Update::DRAW;
                }
            }
        }

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                if let Some(hovered) = self.hovered_index {
                    let item = &self.items[hovered];

                    if item.enabled && item.label != SEPARATOR_LABEL {
                        if item.has_submenu() {
                            // Already handled by hover, but ensure it's open
                            if self.open_item_index != Some(hovered) {
                                self.child_popup = Some(Box::new(interaction::open_submenu(
                                    item,
                                    self.on_close.clone(),
                                )));
                                self.open_item_index = Some(hovered);
                                update |= Update::DRAW;
                            }
                        } else {
                            update |= interaction::handle_item_click(
                                item,
                                hovered,
                                self.on_item_selected.as_ref(),
                                self.on_close.as_ref(),
                            );
                        }
                    }
                } else {
                    update |= interaction::handle_click_outside(
                        child_hovered,
                        self.on_close.as_ref(),
                    );
                }
            }
        }

        update
    }
}
