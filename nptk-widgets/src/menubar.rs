use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
// Overlay system removed for now - using direct rendering instead
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{LayoutNode, LayoutStyle, LengthPercentage, Dimension, StyleNode, Layout};
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Line, Point, Stroke};
use nptk_core::vg::peniko::{Color, Fill, Brush};
use nptk_core::vg::Scene;
use nptk_core::text_render::TextRenderContext;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton, KeyCode, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use crate::menu_popup::{MenuPopup, MenuBarItem as MenuBarItemImpl};

// Re-export MenuBarItem for external use
pub use crate::menu_popup::MenuBarItem;

// Global menu integration removed for now due to API compatibility issues
// #[cfg(feature = "global-menu")]
// use system_tray::menu::{Menu, MenuItem};
// #[cfg(feature = "global-menu")]
// use tokio::sync::mpsc;

// MenuBarItem is now imported from menu_popup module

/// A horizontal menu bar widget with support for hierarchical menus and global menu integration
///
/// ### Theming
/// Styling the menu bar requires the following properties:
/// - `color_background` - The background color of the menu bar.
/// - `color_text` - The text color for menu items.
/// - `color_hovered` - The background color when hovering over items.
/// - `color_selected` - The background color for selected/open items.
/// - `color_disabled` - The text color for disabled items.
/// - `color_border` - The border color for the menu bar.
pub struct MenuBar {
    items: Vec<MenuBarItemImpl>,
    layout_style: MaybeSignal<LayoutStyle>,
    visible: StateSignal<bool>,
    
    // State
    hovered_index: Option<usize>,
    open_menu_index: Option<usize>,
    hovered_submenu_index: Option<usize>,
    text_render_context: TextRenderContext,
    
    // Direct popup rendering (no overlay system)
    popup_data: Option<MenuPopup>,
    
    // Global menu integration (disabled for now)
    // #[cfg(feature = "global-menu")]
    // global_menu_enabled: bool,
    // #[cfg(feature = "global-menu")]
    // menu_sender: Option<mpsc::UnboundedSender<MenuCommand>>,
}

// MenuCommand enum removed for now

impl MenuBar {
    /// Create a new menu bar
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            layout_style: LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::percent(1.0), // Full width
                    Dimension::length(28.0), // Compact height
                ),
                padding: layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(4.0),
                    right: LengthPercentage::length(4.0),
                    top: LengthPercentage::length(2.0),
                    bottom: LengthPercentage::length(2.0),
                },
                flex_direction: nptk_core::layout::FlexDirection::Row,
                align_items: Some(nptk_core::layout::AlignItems::Center),
                ..Default::default()
            }
            .into(),
            visible: StateSignal::new(true),
            hovered_index: None,
            open_menu_index: None,
            hovered_submenu_index: None,
            text_render_context: TextRenderContext::new(),
            popup_data: None,
            
            // Global menu fields removed for now
            // #[cfg(feature = "global-menu")]
            // global_menu_enabled: false,
            // #[cfg(feature = "global-menu")]
            // menu_sender: None,
        }
    }

    /// Add a menu item to the menu bar
    pub fn with_item(mut self, item: MenuBarItemImpl) -> Self {
        self.items.push(item);
        self
    }

    /// Set multiple menu items
    pub fn with_items(mut self, items: Vec<MenuBarItemImpl>) -> Self {
        self.items = items;
        self
    }

    /// Set the layout style for this menu bar
    pub fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = layout_style.into();
        self
    }

    /// Set the visibility of the menu bar
    pub fn with_visible(mut self, visible: impl Into<StateSignal<bool>>) -> Self {
        self.visible = visible.into();
        self
    }

    /// Get the current visibility state
    pub fn is_visible(&self) -> bool {
        *self.visible.get()
    }

    /// Show the menu bar
    pub fn show(&self) {
        self.visible.set(true);
        
        // Global menu integration removed for now
        // #[cfg(feature = "global-menu")]
        // if let Some(sender) = &self.menu_sender {
        //     let _ = sender.send(MenuCommand::ShowMenu);
        // }
    }

    /// Hide the menu bar
    pub fn hide(&self) {
        self.visible.set(false);
        
        // Global menu integration removed for now
        // #[cfg(feature = "global-menu")]
        // if let Some(sender) = &self.menu_sender {
        //     let _ = sender.send(MenuCommand::HideMenu);
        // }
    }

    // Global menu integration methods removed for now - will be re-added once system-tray API is stable

    /// Show a menu popup for the given menu item
    fn show_menu_popup(&mut self, menu_index: usize) {
        if let Some(item) = self.items.get(menu_index) {
            if !item.submenu.is_empty() {
                // Close any existing popup first
                self.popup_data = None;
                self.open_menu_index = None;
                self.hovered_submenu_index = None;
                
                // Create menu popup widget
                let menu_popup = MenuPopup::new()
                    .with_items(item.submenu.clone())
                    .with_on_item_selected(move |_index| {
                        // This will be handled by the popup's on_activate callbacks
                        Update::DRAW
                    })
                    .with_on_close(move || {
                        // Close the popup when an item is selected
                        Update::DRAW
                    });
                
                self.popup_data = Some(menu_popup);
                self.open_menu_index = Some(menu_index);
            }
        }
    }
    

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "MenuBar")
    }

    fn get_item_bounds(&self, layout: &LayoutNode, item_index: usize) -> Rect {
        // Calculate precise item width based on actual text metrics
        let font_size = 14.0;
        let horizontal_padding = 12.0; // Left + right padding per item
        let min_width = 40.0; // Minimum width for very short text
        
        let mut current_x = layout.layout.location.x as f64 + 2.0; // Start with minimal left margin
        
        // Calculate x position by summing widths of previous items
        for i in 0..item_index {
            if let Some(item) = self.items.get(i) {
                // More precise width calculation: character count * average character width + padding
                let text_width = item.label.len() as f64 * (font_size * 0.6); // ~0.6 is average char width ratio
                let item_width = (text_width + horizontal_padding).max(min_width);
                current_x += item_width;
            }
        }
        
        // Calculate this item's width with precise text measurement
        let item_width = if let Some(item) = self.items.get(item_index) {
            let text_width = item.label.len() as f64 * (font_size * 0.6);
            (text_width + horizontal_padding).max(min_width)
        } else {
            min_width
        };
        
        Rect::new(
            current_x,
            layout.layout.location.y as f64,
            current_x + item_width,
            layout.layout.location.y as f64 + layout.layout.size.height as f64,
        )
    }

    fn render_text(text_render_context: &mut TextRenderContext, font_cx: &mut nptk_core::app::font_ctx::FontContext, scene: &mut Scene, text: &str, x: f64, y: f64, color: Color) {
        let font_size = 14.0;
        
        if text.is_empty() {
            return;
        }

        let transform = Affine::translate((x, y));
        
        text_render_context.render_text(
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

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for MenuBar {
    fn widget_id(&self) -> WidgetId {
        self.widget_id()
    }

    fn render(&mut self, scene: &mut Scene, theme: &mut dyn Theme, layout: &LayoutNode, info: &mut AppInfo, _context: AppContext) -> () {
        // Don't render if not visible
        if !self.is_visible() {
            return;
        }

        let widget_theme = theme.of(self.widget_id());
        
        // Pre-calculate theme colors to avoid multiple borrows
        let bg_color = if let Some(ref style) = widget_theme {
            style.get_color("color_background").unwrap_or(Color::from_rgb8(240, 240, 240))
        } else {
            Color::from_rgb8(240, 240, 240)
        };
        
        let border_color = if let Some(ref style) = widget_theme {
            style.get_color("color_border").unwrap_or(Color::from_rgb8(200, 200, 200))
        } else {
            Color::from_rgb8(200, 200, 200)
        };
        
        let text_color = if let Some(ref style) = widget_theme {
            style.get_color("color_text").unwrap_or(Color::BLACK)
        } else {
            Color::BLACK
        };
        
        let disabled_color = if let Some(ref style) = widget_theme {
            style.get_color("color_disabled").unwrap_or(Color::from_rgb8(150, 150, 150))
        } else {
            Color::from_rgb8(150, 150, 150)
        };
        
        let selected_color = if let Some(ref style) = widget_theme {
            style.get_color("color_selected").unwrap_or(Color::from_rgb8(70, 130, 255))
        } else {
            Color::from_rgb8(70, 130, 255)
        };
        
        let hovered_color = if let Some(ref style) = widget_theme {
            style.get_color("color_hovered").unwrap_or(Color::from_rgb8(220, 220, 220))
        } else {
            Color::from_rgb8(220, 220, 220)
        };
        
        // Draw menu bar background
        let menu_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &menu_rect);

        // Draw border
        let stroke = Stroke::new(1.0);
        scene.stroke(&stroke, Affine::IDENTITY, border_color, None, &menu_rect);

        // Draw menu items
        for (i, item) in self.items.iter().enumerate() {
            let item_bounds = self.get_item_bounds(layout, i);
            
            // Determine item colors using pre-calculated colors
            let (item_text_color, item_bg_color) = if !item.enabled {
                (disabled_color, Color::TRANSPARENT)
            } else if Some(i) == self.open_menu_index {
                (text_color, selected_color)
            } else if Some(i) == self.hovered_index {
                (text_color, hovered_color)
            } else {
                (text_color, Color::TRANSPARENT)
            };

            // Draw item background if needed
            if item_bg_color != Color::TRANSPARENT {
                let item_rounded = RoundedRect::new(
                    item_bounds.x0,
                    item_bounds.y0,
                    item_bounds.x1,
                    item_bounds.y1,
                    RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
                );
                scene.fill(Fill::NonZero, Affine::IDENTITY, item_bg_color, None, &item_rounded);
            }

            // Draw item text centered in the item bounds
            let text_x = item_bounds.x0 + 6.0; // Small left padding
            let text_y = item_bounds.y0 + 2.0; // Adjust for proper baseline
            Self::render_text(&mut self.text_render_context, &mut info.font_context, scene, &item.label, text_x, text_y, item_text_color);

            // Draw submenu indicator below the text if item has submenu
            if item.has_submenu() {
                let arrow_x = item_bounds.x0 + (item_bounds.width() / 2.0); // Center horizontally
                let arrow_y = item_bounds.y1 - 6.0; // Position at bottom with small margin
                
                // Draw small down arrow below text
                let arrow_size = 2.0;
                let arrow_stroke = Stroke::new(1.0);
                
                // Simple down arrow (V shape)
                scene.stroke(
                    &arrow_stroke,
                    Affine::IDENTITY,
                    item_text_color,
                    None,
                    &Line::new(
                        Point::new(arrow_x - arrow_size, arrow_y - arrow_size),
                        Point::new(arrow_x, arrow_y),
                    ),
                );
                scene.stroke(
                    &arrow_stroke,
                    Affine::IDENTITY,
                    item_text_color,
                    None,
                    &Line::new(
                        Point::new(arrow_x, arrow_y),
                        Point::new(arrow_x + arrow_size, arrow_y - arrow_size),
                    ),
                );
            }
        }

        // Render popup if open
        if let Some(open_index) = self.open_menu_index {
            // Calculate position below the menu item first
            let item_bounds = self.get_item_bounds(layout, open_index);
            let popup_x = item_bounds.x0;
            let popup_y = item_bounds.y1;
            
            // Create layout node for the popup
            let mut popup_layout = LayoutNode {
                layout: Layout::default(),
                children: Vec::new(),
            };
            
            // Set position and size based on popup's calculated size
            if let Some(ref mut popup) = self.popup_data {
                let (popup_width, popup_height) = popup.calculate_size();
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;
                
                // Now render the popup
                popup.render(scene, theme, &popup_layout, info, _context);
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Don't process events if not visible
        if !self.is_visible() {
            return update;
        }

        // Global menu update removed for now

        // Get mouse position
        let cursor_pos = info.cursor_pos;
        
        // Check hover state for main menu items
        let old_hovered = self.hovered_index;
        let old_submenu_hovered = self.hovered_submenu_index;
        self.hovered_index = None;
        self.hovered_submenu_index = None;
        
        if let Some(pos) = cursor_pos {
            // Check main menu items
            for i in 0..self.items.len() {
                let item_bounds = self.get_item_bounds(layout, i);
                if pos.x as f64 >= item_bounds.x0
                    && pos.x as f64 <= item_bounds.x1
                    && pos.y as f64 >= item_bounds.y0
                    && pos.y as f64 <= item_bounds.y1
                {
                    self.hovered_index = Some(i);
                    
                    // If a menu is already open and we hover over a different menu item,
                    // switch to that menu (standard GUI behavior)
                    if self.open_menu_index.is_some() && self.open_menu_index != Some(i) {
                        let item = &self.items[i];
                        if item.enabled && item.has_submenu() {
                            // Close current popup and show new one
                            self.popup_data = None;
                            self.open_menu_index = None;
                            self.hovered_submenu_index = None;
                            
                            // Show new popup
                            self.show_menu_popup(i);
                            update |= Update::DRAW;
                        }
                    }
                    break;
                }
            }
        }

        if old_hovered != self.hovered_index || old_submenu_hovered != self.hovered_submenu_index {
            update |= Update::DRAW;
        }

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                if let Some(hovered) = self.hovered_index {
                    let item = &self.items[hovered];
                    
                    if item.enabled {
                        if item.has_submenu() {
                            // Toggle submenu
                            if self.open_menu_index == Some(hovered) {
                                // Close current popup
                                self.popup_data = None;
                                self.open_menu_index = None;
                                self.hovered_submenu_index = None;
                            } else {
                                // Show new popup
                                self.show_menu_popup(hovered);
                            }
                            update |= Update::DRAW;
                        } else {
                            // Execute item callback
                            if let Some(ref callback) = item.on_activate {
                                update |= callback();
                            }
                        }
                    }
                } else {
                    // Click outside - close popup
                    if self.popup_data.is_some() {
                        self.popup_data = None;
                        self.open_menu_index = None;
                        self.hovered_submenu_index = None;
                        update |= Update::DRAW;
                    }
                }
            }
        }

        // Update popup if open
        if let Some(open_index) = self.open_menu_index {
            // Calculate position below the menu item first
            let item_bounds = self.get_item_bounds(layout, open_index);
            let popup_x = item_bounds.x0;
            let popup_y = item_bounds.y1;
            
            // Create layout node for the popup
            let mut popup_layout = LayoutNode {
                layout: Layout::default(),
                children: Vec::new(),
            };
            
            // Set position and size based on popup's calculated size
            if let Some(ref mut popup) = self.popup_data {
                let (popup_width, popup_height) = popup.calculate_size();
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;
                
                // Now update the popup
                let popup_update = popup.update(&popup_layout, _context, info);
                update |= popup_update;
                
                // Check if popup wants to close itself
                if popup_update.contains(Update::DRAW) {
                    // This is a simple way to detect if popup was closed
                    // In a real implementation, you'd have a proper close signal
                }
            }
        }

        // Handle keyboard shortcuts
        for (_, key_event) in &info.keys {
            if key_event.state == ElementState::Pressed {
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.open_menu_index.is_some() {
                            self.popup_data = None;
                            self.open_menu_index = None;
                            self.hovered_submenu_index = None;
                            update |= Update::DRAW;
                        }
                    }
                    PhysicalKey::Code(KeyCode::F10) => {
                        // Toggle menu bar visibility
                        let visible = !self.is_visible();
                        self.visible.set(visible);
                        update |= Update::DRAW | Update::LAYOUT;
                    }
                    _ => {}
                }
            }
        }

        update
    }

    fn layout_style(&self) -> StyleNode {
        // Hide the widget by setting height to 0 if not visible
        let mut style = self.layout_style.get().clone();
        if !self.is_visible() {
            style.size.y = Dimension::length(0.0);
        }
        StyleNode {
            style,
            children: vec![],
        }
    }
}

impl WidgetLayoutExt for MenuBar {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
