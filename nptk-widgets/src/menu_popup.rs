// SPDX-License-Identifier: MIT OR Apache-2.0

//! Menu popup widget for dropdown menus
//! 
//! This widget is designed to be used as overlay content for menu dropdowns.
//! It provides a clean, themed popup menu that can be positioned anywhere on screen.

use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, LengthPercentage, Dimension, StyleNode};
use std::sync::Arc;
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Line, Point, Stroke};
use nptk_core::vg::peniko::{Color, Fill, Brush};
use nptk_core::vg::Scene;
use nptk_core::text_render::TextRenderContext;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

/// A popup menu widget that displays a list of menu items
pub struct MenuPopup {
    /// The menu items to display
    items: Vec<MenuBarItem>,
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
                    top: LengthPercentage::length(4.0),
                    bottom: LengthPercentage::length(4.0),
                },
                ..Default::default()
            }
            .into(),
            hovered_index: None,
            text_render_context: TextRenderContext::new(),
            on_item_selected: None,
            on_close: None,
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

    /// Get the widget ID
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "MenuPopup")
    }

    /// Calculate the size needed for the popup based on items
    pub fn calculate_size(&self) -> (f64, f64) {
        let item_height = 24.0;
        let padding = 8.0; // Top and bottom padding
        let min_width = 120.0;
        let max_width = 400.0; // Increased max width for longer text
        let text_padding = 16.0; // Left and right padding for text
        
        // Calculate height based on number of items
        let height = (self.items.len() as f64 * item_height) + padding;
        
        // Calculate width based on longest item text + shortcut
        let mut max_total_width: f64 = min_width;
        for item in &self.items {
            if item.label != "---" { // Skip separators
                // Calculate text width (rough estimate: 8 pixels per character)
                let text_width: f64 = item.label.len() as f64 * 8.0;
                
                // Calculate shortcut width if present
                let shortcut_width: f64 = if let Some(ref shortcut) = item.shortcut {
                    shortcut.len() as f64 * 7.0 // Slightly smaller font for shortcuts
                } else {
                    0.0
                };
                
                // For right-aligned shortcuts, we need space for:
                // - text width + left padding
                // - minimum gap between text and shortcut (to avoid overlap)
                // - shortcut width + right padding
                let min_gap = 20.0; // Minimum gap between text and shortcut
                let total_width = text_width + text_padding + min_gap + shortcut_width + 8.0; // 8.0 is right padding
                max_total_width = max_total_width.max(total_width);
            }
        }
        
        let width: f64 = max_total_width.min(max_width);
        
        (width, height)
    }

    /// Render text helper
    fn render_text(text_render_context: &mut TextRenderContext, font_cx: &mut nptk_core::app::font_ctx::FontContext, scene: &mut Scene, text: &str, x: f64, y: f64, color: Color) {
        let font_size = 14.0;
        
        if text.is_empty() {
            return;
        }

        let transform = Affine::translate((x, y));
        
        // Try to render text, but don't panic if font context is not available
        let _ = text_render_context.render_text(
            font_cx,
            scene,
            text,
            None, // No specific font, use default
            font_size,
            Brush::Solid(color),
            transform,
            true, // hinting
        );
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
        self.widget_id()
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

    fn render(&mut self, scene: &mut Scene, theme: &mut dyn Theme, layout: &LayoutNode, info: &mut AppInfo, _context: AppContext) {
        let widget_theme = theme.of(self.widget_id());
        
        // Pre-calculate theme colors with proper fallbacks
        let bg_color = if let Some(ref style) = widget_theme {
            style.get_color("color_background")
                .unwrap_or_else(|| theme.defaults().container().background())
        } else {
            theme.defaults().container().background()
        };
        
        let border_color = if let Some(ref style) = widget_theme {
            style.get_color("color_border")
                .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)) // Light gray border
        } else {
            Color::from_rgb8(200, 200, 200) // Light gray border
        };
        
        let text_color = if let Some(ref style) = widget_theme {
            style.get_color("color_text")
                .unwrap_or_else(|| theme.defaults().text().foreground())
        } else {
            theme.defaults().text().foreground()
        };
        
        let disabled_color = if let Some(ref style) = widget_theme {
            style.get_color("color_disabled")
                .unwrap_or_else(|| theme.defaults().interactive().disabled())
        } else {
            theme.defaults().interactive().disabled()
        };
        
        let hovered_color = if let Some(ref style) = widget_theme {
            style.get_color("color_hovered")
                .unwrap_or_else(|| theme.defaults().interactive().hover())
        } else {
            theme.defaults().interactive().hover()
        };

        // Calculate popup size
        let (popup_width, popup_height) = self.calculate_size();
        
        // Draw popup background
        let popup_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            layout.layout.location.x as f64 + popup_width,
            layout.layout.location.y as f64 + popup_height,
        );

        let popup_rounded = RoundedRect::new(
            popup_rect.x0,
            popup_rect.y0,
            popup_rect.x1,
            popup_rect.y1,
            RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
        );
        scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &popup_rounded);

        // Draw border
        let stroke = Stroke::new(1.0);
        scene.stroke(&stroke, Affine::IDENTITY, border_color, None, &popup_rounded);

        // Draw menu items
        let item_height = 24.0;
        for (i, item) in self.items.iter().enumerate() {
            let item_y = popup_rect.y0 + 4.0 + (i as f64 * item_height);
            let item_rect = Rect::new(
                popup_rect.x0,
                item_y,
                popup_rect.x1,
                item_y + item_height,
            );

            // Determine item colors
            let (item_text_color, item_bg_color) = if !item.enabled {
                (disabled_color, Color::TRANSPARENT)
            } else if Some(i) == self.hovered_index {
                (text_color, hovered_color)
            } else {
                (text_color, Color::TRANSPARENT)
            };

            // Draw item background if needed
            if item_bg_color != Color::TRANSPARENT {
                let item_rounded = RoundedRect::new(
                    item_rect.x0 + 2.0,
                    item_rect.y0,
                    item_rect.x1 - 2.0,
                    item_rect.y1,
                    RoundedRectRadii::new(2.0, 2.0, 2.0, 2.0),
                );
                scene.fill(Fill::NonZero, Affine::IDENTITY, item_bg_color, None, &item_rounded);
            }

            // Draw item content
            if item.label != "---" { // Skip separators
                // Draw item text
                let text_x = item_rect.x0 + 8.0;
                let text_y = item_rect.y0 + 2.0;
                Self::render_text(&mut self.text_render_context, &mut info.font_context, scene, &item.label, text_x, text_y, item_text_color);
                
                // Draw keyboard shortcut if present
                if let Some(ref shortcut) = item.shortcut {
                    // Calculate shortcut width
                    let shortcut_width = shortcut.len() as f64 * 7.0; // Same estimate as in calculate_size
                    
                    // Position shortcut at the right edge with padding
                    let shortcut_x = item_rect.x1 - 8.0 - shortcut_width;
                    
                    let shortcut_color = Color::from_rgb8(120, 120, 120); // Dimmed color
                    Self::render_text(&mut self.text_render_context, &mut info.font_context, scene, shortcut, shortcut_x, text_y, shortcut_color);
                }
            } else {
                // Draw separator line
                let sep_stroke = Stroke::new(1.0);
                let sep_y = item_rect.y0 + (item_height / 2.0);
                scene.stroke(
                    &sep_stroke,
                    Affine::IDENTITY,
                    Color::from_rgb8(200, 200, 200),
                    None,
                    &Line::new(
                        Point::new(item_rect.x0 + 8.0, sep_y),
                        Point::new(item_rect.x1 - 8.0, sep_y),
                    ),
                );
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Get mouse position
        let cursor_pos = info.cursor_pos;
        
        // Check hover state
        let old_hovered = self.hovered_index;
        self.hovered_index = None;
        
        if let Some(pos) = cursor_pos {
            let (popup_width, popup_height) = self.calculate_size();
            let popup_rect = Rect::new(
                layout.layout.location.x as f64,
                layout.layout.location.y as f64,
                layout.layout.location.x as f64 + popup_width,
                layout.layout.location.y as f64 + popup_height,
            );
            
            // Check if mouse is within popup bounds
            if pos.x as f32 >= popup_rect.x0 as f32
                && pos.x as f32 <= popup_rect.x1 as f32
                && pos.y as f32 >= popup_rect.y0 as f32
                && pos.y as f32 <= popup_rect.y1 as f32
            {
                // Find which item is being hovered
                let item_height = 24.0;
                let relative_y = pos.y as f32 - popup_rect.y0 as f32 - 4.0; // Account for padding
                let item_index = (relative_y / item_height) as usize;
                
                if item_index < self.items.len() {
                    let item = &self.items[item_index];
                    if item.enabled && item.label != "---" {
                        self.hovered_index = Some(item_index);
                    }
                }
            }
        }

        if old_hovered != self.hovered_index {
            update |= Update::DRAW;
        }

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                if let Some(hovered) = self.hovered_index {
                    let item = &self.items[hovered];
                    
                    if item.enabled && item.label != "---" {
                        // Execute item callback
                        if let Some(ref callback) = item.on_activate {
                            update |= callback();
                        }
                        
                        // Notify parent of selection
                        if let Some(ref callback) = self.on_item_selected {
                            update |= callback(hovered);
                        }
                        
                        // Close popup
                        if let Some(ref callback) = self.on_close {
                            update |= callback();
                        }
                    }
                } else {
                    // Click outside - close popup
                    if let Some(ref callback) = self.on_close {
                        update |= callback();
                    }
                }
            }
        }

        update
    }
}
