use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{LayoutNode, LayoutStyle, LengthPercentage, Dimension, StyleNode};
use std::sync::Arc;
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke, Line, Point};
use nptk_core::vg::peniko::{Fill, Color};
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton, KeyCode, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nptk_core::skrifa::MetadataProvider;

// Global menu integration removed for now due to API compatibility issues
// #[cfg(feature = "global-menu")]
// use system_tray::menu::{Menu, MenuItem};
// #[cfg(feature = "global-menu")]
// use tokio::sync::mpsc;

/// Represents a menu item in the menu bar
#[derive(Clone)]
pub struct MenuBarItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub submenu: Vec<MenuBarItem>,
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
    items: Vec<MenuBarItem>,
    layout_style: MaybeSignal<LayoutStyle>,
    visible: StateSignal<bool>,
    
    // State
    hovered_index: Option<usize>,
    open_menu_index: Option<usize>,
    hovered_submenu_index: Option<usize>,
    
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
            
            // Global menu fields removed for now
            // #[cfg(feature = "global-menu")]
            // global_menu_enabled: false,
            // #[cfg(feature = "global-menu")]
            // menu_sender: None,
        }
    }

    /// Add a menu item to the menu bar
    pub fn with_item(mut self, item: MenuBarItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set multiple menu items
    pub fn with_items(mut self, items: Vec<MenuBarItem>) -> Self {
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

    fn render_text(&self, scene: &mut Scene, text: &str, x: f64, y: f64, color: Color, info: &AppInfo) {
        let font_size = 14.0;
        
        // Get default font from the font context
        let font = info.font_context.default_font().clone();
        
        let font_ref = {
            let file_ref = nptk_core::skrifa::raw::FileRef::new(font.data.as_ref()).expect("Failed to load font data");
            match file_ref {
                nptk_core::skrifa::raw::FileRef::Font(font) => Some(font),
                nptk_core::skrifa::raw::FileRef::Collection(collection) => collection.get(font.index).ok(),
            }
        }
        .expect("Failed to load font reference");

        let location = font_ref.axes().location::<&[nptk_core::skrifa::setting::VariationSetting; 0]>(&[]);
        let glyph_metrics = font_ref.glyph_metrics(nptk_core::skrifa::instance::Size::new(font_size), &location);
        let charmap = font_ref.charmap();

        let mut pen_x = x as f32;
        let pen_y = y as f32 + font_size;

        scene
            .draw_glyphs(&font)
            .font_size(font_size)
            .brush(&nptk_core::vg::peniko::Brush::Solid(color))
            .normalized_coords(bytemuck::cast_slice(location.coords()))
            .hint(true)
            .draw(
                &nptk_core::vg::peniko::Style::Fill(Fill::NonZero),
                text.chars().filter_map(|c| {
                    let gid = charmap.map(c).unwrap_or_default();
                    let advance = glyph_metrics.advance_width(gid).unwrap_or_default();
                    let x = pen_x;
                    pen_x += advance;

                    Some(nptk_core::vg::Glyph {
                        id: gid.to_u32(),
                        x,
                        y: pen_y,
                    })
                }),
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

    fn render(&mut self, scene: &mut Scene, theme: &mut dyn Theme, layout: &LayoutNode, info: &AppInfo, _context: AppContext) -> () {
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
            self.render_text(scene, &item.label, text_x, text_y, item_text_color, info);

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

        // Render open submenu (basic implementation without overlay system)
        if let Some(open_index) = self.open_menu_index {
            if let Some(open_item) = self.items.get(open_index) {
                if !open_item.submenu.is_empty() {
                    let menu_bounds = self.get_item_bounds(layout, open_index);
                    let submenu_width = 200.0;
                    let item_height = 24.0;
                    let submenu_height = open_item.submenu.len() as f64 * item_height;
                    
                    let submenu_rect = Rect::new(
                        menu_bounds.x0,
                        menu_bounds.y1, // Start below the menu item
                        menu_bounds.x0 + submenu_width,
                        menu_bounds.y1 + submenu_height,
                    );

                    // Draw submenu background
                    let submenu_rounded = RoundedRect::new(
                        submenu_rect.x0,
                        submenu_rect.y0,
                        submenu_rect.x1,
                        submenu_rect.y1,
                        RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
                    );
                    scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &submenu_rounded);
                    
                    // Draw submenu border
                    let stroke = Stroke::new(1.0);
                    scene.stroke(&stroke, Affine::IDENTITY, border_color, None, &submenu_rounded);

                    // Draw submenu items
                    for (i, submenu_item) in open_item.submenu.iter().enumerate() {
                        let item_y = submenu_rect.y0 + (i as f64 * item_height);
                        let item_rect = Rect::new(
                            submenu_rect.x0,
                            item_y,
                            submenu_rect.x1,
                            item_y + item_height,
                        );

                        // Determine submenu item colors
                        let (submenu_text_color, submenu_bg_color) = if !submenu_item.enabled {
                            (disabled_color, Color::TRANSPARENT)
                        } else if Some(i) == self.hovered_submenu_index {
                            (text_color, hovered_color) // Highlight hovered submenu item
                        } else {
                            (text_color, Color::TRANSPARENT)
                        };

                        // Draw submenu item background if needed
                        if submenu_bg_color != Color::TRANSPARENT {
                            let submenu_item_rounded = RoundedRect::new(
                                item_rect.x0 + 2.0,
                                item_rect.y0,
                                item_rect.x1 - 2.0,
                                item_rect.y1,
                                RoundedRectRadii::new(2.0, 2.0, 2.0, 2.0),
                            );
                            scene.fill(Fill::NonZero, Affine::IDENTITY, submenu_bg_color, None, &submenu_item_rounded);
                        }

                        // Draw submenu item text
                        if submenu_item.label != "---" { // Skip separators
                            let submenu_text_x = item_rect.x0 + 8.0;
                            let submenu_text_y = item_rect.y0 + 2.0;
                            self.render_text(scene, &submenu_item.label, submenu_text_x, submenu_text_y, submenu_text_color, info);
                            
                            // Draw keyboard shortcut if present
                            if let Some(ref shortcut) = submenu_item.shortcut {
                                let shortcut_x = item_rect.x1 - 60.0; // Right-aligned
                                let shortcut_color = Color::from_rgb8(120, 120, 120); // Dimmed color
                                self.render_text(scene, shortcut, shortcut_x, submenu_text_y, shortcut_color, info);
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
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &AppInfo) -> Update {
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
            // First check if hovering over submenu items
            let mut hovering_submenu = false;
            if let Some(open_index) = self.open_menu_index {
                if let Some(open_item) = self.items.get(open_index) {
                    if !open_item.submenu.is_empty() {
                        let menu_bounds = self.get_item_bounds(layout, open_index);
                        let submenu_width = 200.0;
                        let item_height = 24.0;
                        
                        for (i, _submenu_item) in open_item.submenu.iter().enumerate() {
                            let item_y = menu_bounds.y1 + (i as f64 * item_height);
                            let item_rect = Rect::new(
                                menu_bounds.x0,
                                item_y,
                                menu_bounds.x0 + submenu_width,
                                item_y + item_height,
                            );
                            
                            if pos.x as f64 >= item_rect.x0
                                && pos.x as f64 <= item_rect.x1
                                && pos.y as f64 >= item_rect.y0
                                && pos.y as f64 <= item_rect.y1
                            {
                                self.hovered_submenu_index = Some(i);
                                hovering_submenu = true;
                                break;
                            }
                        }
                    }
                }
            }
            
            // If not hovering over submenu, check main menu items
            if !hovering_submenu {
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
                                self.open_menu_index = Some(i);
                                update |= Update::DRAW;
                            }
                        }
                        break;
                    }
                }
            }
        }

        if old_hovered != self.hovered_index || old_submenu_hovered != self.hovered_submenu_index {
            update |= Update::DRAW;
        }

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left && *state == ElementState::Pressed {
                let mut handled = false;
                
                // Check if clicking on a submenu item first
                if let Some(open_index) = self.open_menu_index {
                    if let Some(open_item) = self.items.get(open_index) {
                        if !open_item.submenu.is_empty() {
                            let menu_bounds = self.get_item_bounds(layout, open_index);
                            let submenu_width = 200.0;
                            let item_height = 24.0;
                            
                            for (i, submenu_item) in open_item.submenu.iter().enumerate() {
                                let item_y = menu_bounds.y1 + (i as f64 * item_height);
                                let item_rect = Rect::new(
                                    menu_bounds.x0,
                                    item_y,
                                    menu_bounds.x0 + submenu_width,
                                    item_y + item_height,
                                );
                                
                                if let Some(pos) = cursor_pos {
                                    if pos.x as f64 >= item_rect.x0
                                        && pos.x as f64 <= item_rect.x1
                                        && pos.y as f64 >= item_rect.y0
                                        && pos.y as f64 <= item_rect.y1
                                        && submenu_item.enabled
                                        && submenu_item.label != "---"
                                    {
                                        // Execute submenu item callback
                                        if let Some(ref callback) = submenu_item.on_activate {
                                            update |= callback();
                                        }
                                        self.open_menu_index = None;
                                        handled = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                
                // If not handled by submenu, check main menu items
                if !handled {
                    if let Some(hovered) = self.hovered_index {
                        let item = &self.items[hovered];
                        
                        if item.enabled {
                            if item.has_submenu() {
                                // Toggle submenu
                                if self.open_menu_index == Some(hovered) {
                                    self.open_menu_index = None;
                                } else {
                                    self.open_menu_index = Some(hovered);
                                }
                                update |= Update::DRAW;
                            } else if let Some(ref callback) = item.on_activate {
                                // Execute callback
                                update |= callback();
                                self.open_menu_index = None;
                            }
                        }
                    } else {
                        // Click outside menu - close any open submenu
                        if self.open_menu_index.is_some() {
                            self.open_menu_index = None;
                            update |= Update::DRAW;
                        }
                    }
                }
            }
        }

        // Handle keyboard shortcuts
        for (_, key_event) in &info.keys {
            if key_event.state == ElementState::Pressed {
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.open_menu_index.is_some() {
                            self.open_menu_index = None;
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
