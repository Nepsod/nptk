// SPDX-License-Identifier: LGPL-3.0-only
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
// Overlay system removed for now - using direct rendering instead
#[cfg(feature = "global-menu")]
use super::common::platform;
#[cfg(feature = "global-menu")]
use super::dbus::{Bridge, MenuSnapshot, RemoteMenuNode};
use crate::menu_popup::MenuPopup;
use nptk_core::menu::unified::{MenuTemplate, MenuItem, MenuContext};
use nptk_core::menu::templates::{init_edit_commands, init_view_menu};
use nptk_core::menu::render::render_menu;
use nptk_core::menu::manager::MenuManager;
use nptk_core::menu::commands::MenuCommand;
#[cfg(feature = "global-menu")]
use log::error;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{
    Dimension, Display, Layout, LayoutNode, LayoutStyle, LengthPercentage, LengthPercentageAuto,
    StyleNode,
};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{
    Affine, Line, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, KeyCode, MouseButton, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
#[cfg(feature = "global-menu")]
use std::collections::hash_map::DefaultHasher;
#[cfg(feature = "global-menu")]
use std::collections::HashMap;
#[cfg(feature = "global-menu")]
use std::hash::{Hash, Hasher};
#[cfg(feature = "global-menu")]
use std::sync::Arc;
use async_trait::async_trait;

// MenuBarItem is re-exported from the parent module

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
    // Menu templates (unified system)
    menu_templates: Vec<MenuTemplate>,
    // Menu manager for command routing
    menu_manager: Option<MenuManager>,
    // Menu context for dynamic enabling/disabling
    menu_context: MenuContext,
    #[cfg(feature = "global-menu")]
    global_menu_bridge: Option<Bridge>,
    #[cfg(feature = "global-menu")]
    global_menu_actions: HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>,
    #[cfg(feature = "global-menu")]
    global_menu_signature: u64,
    #[cfg(feature = "global-menu")]
    last_window_id: Option<u64>,
    #[cfg(feature = "global-menu")]
    importer_detected: StateSignal<bool>,
    #[cfg(feature = "global-menu")]
    global_menu_enabled: bool,
    layout_style: MaybeSignal<LayoutStyle>,
    visible: StateSignal<bool>,
    previous_visible: bool,
    /// Overrides the auto-hide behavior from global menu integration
    user_visibility_override: Option<bool>,

    // State
    hovered_index: Option<usize>,
    open_menu_index: Option<usize>,
    hovered_submenu_index: Option<usize>,
    text_render_context: TextRenderContext,

    // Menu template for currently open popup
    open_template: Option<MenuTemplate>,
    
    // Submenu support
    open_submenu_index: Option<usize>,
    open_submenu_template: Option<MenuTemplate>,
    // Global menu integration (disabled for now)
    // #[cfg(feature = "global-menu")]
    // global_menu_enabled: bool,
    // #[cfg(feature = "global-menu")]
    // menu_sender: Option<mpsc::UnboundedSender<MenuCommand>>,
}

#[cfg(feature = "global-menu")]
use super::adapter::menu_template_to_snapshot;

#[cfg(target_os = "linux")]
fn current_window_x11_id(info: &AppInfo) -> Option<u32> {
    info.window_x11_id()
}

#[cfg(feature = "global-menu")]
fn is_wayland_session() -> bool {
    platform::is_wayland_session()
}

#[cfg(not(feature = "global-menu"))]
fn is_wayland_session() -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
fn current_window_x11_id(_info: &AppInfo) -> Option<u32> {
    None
}

// MenuCommand enum removed for now

impl MenuBar {
    /// Create a new menu bar
    pub fn new() -> Self {
        Self {
            menu_templates: Vec::new(),
            menu_manager: None,
            menu_context: MenuContext::new(),
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
            previous_visible: true,
            #[cfg(feature = "global-menu")]
            global_menu_bridge: None,
            #[cfg(feature = "global-menu")]
            global_menu_actions: HashMap::new(),
            #[cfg(feature = "global-menu")]
            global_menu_signature: 0,
            #[cfg(feature = "global-menu")]
            last_window_id: None,
            #[cfg(feature = "global-menu")]
            importer_detected: StateSignal::new(false),
            #[cfg(feature = "global-menu")]
            global_menu_enabled: true,
            hovered_index: None,
            open_menu_index: None,
            hovered_submenu_index: None,
            text_render_context: TextRenderContext::new(),
            open_template: None,
            open_submenu_index: None,
            open_submenu_template: None,
            user_visibility_override: None,
        }
    }

    /// Add a menu template to the menu bar
    pub fn with_template(mut self, template: MenuTemplate) -> Self {
        self.menu_templates.push(template);
        self
    }

    /// Set multiple menu templates
    pub fn with_templates(mut self, templates: Vec<MenuTemplate>) -> Self {
        self.menu_templates = templates;
        self
    }

    /// Set the menu manager for command routing
    pub fn with_menu_manager(mut self, manager: MenuManager) -> Self {
        self.menu_manager = Some(manager);
        self
    }

    /// Update the menu context for dynamic enabling/disabling
    pub fn set_menu_context(&mut self, context: MenuContext) {
        self.menu_context = context;
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

    #[cfg(feature = "global-menu")]
    /// Enable integration with the system-wide global menu via StatusNotifierItem + DBusMenu.
    pub fn with_global_menu(mut self) -> Self {
        self.global_menu_enabled = true;
        self.enable_global_menu();
        self
    }

    #[cfg(not(feature = "global-menu"))]
    /// Placeholder for the with_global_menu method when global-menu feature is disabled.
    pub fn with_global_menu(mut self) -> Self {
        log::warn!("with_global_menu called when global-menu feature is disabled");
        self
    }

    #[cfg(feature = "global-menu")]
    /// Disable integration with the system-wide global menu.
    pub fn without_global_menu(mut self) -> Self {
        self.global_menu_enabled = false;
        // If bridge was already started, we can't easily stop it, but we can stop updating it
        // Ideally we would drop the bridge here if it was an Option<Bridge> that we own fully
        self
    }

    #[cfg(not(feature = "global-menu"))]
    /// Placeholder for the without_global_menu method when global-menu feature is disabled.
    pub fn without_global_menu(mut self) -> Self {
        log::warn!("without_global_menu called when global-menu feature is disabled");
        self
    }

    #[cfg(feature = "global-menu")]
    /// Lazily start the background bridge that mirrors DBus menus into this menu bar.
    pub fn enable_global_menu(&mut self) {
        if self.global_menu_bridge.is_none() {
            self.global_menu_bridge = Bridge::start();
            if self.global_menu_bridge.is_none() {
                error!("Failed to initialize global menu bridge");
            }
        }
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
        // Try unified menu template first
        if menu_index < self.menu_templates.len() {
            let template = self.menu_templates[menu_index].clone();
            
            // Apply context-aware initialization
            let mut template = template;
            
            // Initialize Edit menu items if this is an Edit menu
            if template.id == "Edit" || template.id == "edit_menu" {
                init_edit_commands(&mut template, &self.menu_context);
            }
            
            // Initialize View menu items if this is a View menu
            if template.id == "View" || template.id == "view_menu" {
                init_view_menu(&mut template, &self.menu_context);
            }
            
            self.open_template = Some(template);
            self.open_menu_index = Some(menu_index);
        }
    }

    /// Get the label for a menu template at the given index
    fn get_template_label(&self, index: usize) -> Option<&str> {
        self.menu_templates.get(index).map(|template| template.id.as_str())
    }

    fn get_item_bounds(&self, layout: &LayoutNode, item_index: usize) -> Rect {
        // Calculate precise item width based on actual text metrics
        let font_size = 14.0;
        let horizontal_padding = 12.0; // Left + right padding per item
        let min_width = 40.0; // Minimum width for very short text

        let mut current_x = layout.layout.location.x as f64 + 2.0; // Start with minimal left margin

        // Calculate x position by summing widths of previous items
        for i in 0..item_index {
            if let Some(label) = self.get_template_label(i) {
                let text_width = label.len() as f64 * (font_size * 0.6);
                let item_width = (text_width + horizontal_padding).max(min_width);
                current_x += item_width;
            }
        }

        // Calculate this item's width with precise text measurement
        let item_width = if let Some(label) = self.get_template_label(item_index) {
            let text_width = label.len() as f64 * (font_size * 0.6);
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

    fn render_text(
        text_render_context: &mut TextRenderContext,
        font_cx: &mut nptk_core::app::font_ctx::FontContext,
        graphics: &mut dyn Graphics,
        text: &str,
        x: f64,
        y: f64,
        color: Color,
    ) {
        let font_size = 14.0;

        if text.is_empty() {
            return;
        }

        let transform = Affine::translate((x, y));

        text_render_context.render_text(
            font_cx,
            graphics,
            text,
            None, // No specific font, use default
            font_size,
            Brush::Solid(color),
            transform,
            true, // hinting
            None, // No width constraint for menu items
        );
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for MenuBar {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "MenuBar")
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) -> () {
        // Don't render if not visible
        // Note: If importer is detected and user presses F10, importer_detected is cleared,
        // so we just need to check is_visible()
        if !self.is_visible() {
            return;
        }

        // Pre-calculate theme colors with proper fallbacks
        let bg_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorBackground,
            )
            .unwrap_or_else(|| Color::from_rgb8(240, 240, 240));

        let border_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorBorder,
            )
            .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)); // Light gray border

        let text_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorText,
            )
            .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));

        let disabled_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorDisabled,
            )
            .unwrap_or_else(|| Color::from_rgb8(150, 150, 150));

        let selected_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorMenuSelected,
            )
            .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));

        let hovered_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorMenuHovered,
            )
            .unwrap_or_else(|| Color::from_rgb8(180, 180, 180));

        // Draw menu bar background
        let menu_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &menu_rect.to_path(0.1),
        );

        // Draw border
        let stroke = Stroke::new(1.0);
        graphics.stroke(
            &stroke,
            Affine::IDENTITY,
            &Brush::Solid(border_color),
            None,
            &menu_rect.to_path(0.1),
        );

        // Draw menu items
        for i in 0..self.menu_templates.len() {
            let item_bounds = self.get_item_bounds(layout, i);
            
            // Get label and enabled state from template
            let template = &self.menu_templates[i];
            let label = self.get_template_label(i).unwrap_or(template.id.as_str());
            let enabled = true;
            let has_submenu = !template.items.is_empty();

            // Determine item colors using pre-calculated colors
            let (item_text_color, item_bg_color) = if !enabled {
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
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(item_bg_color),
                    None,
                    &item_rounded.to_path(0.1),
                );
            }

            // Draw item text centered in the item bounds
            let text_x = item_bounds.x0 + 6.0; // Small left padding
            let text_y = item_bounds.y0 + 2.0; // Adjust for proper baseline
            // Create temporary text render context to avoid borrow conflicts
            let mut temp_text_render_context = TextRenderContext::new();
            Self::render_text(
                &mut temp_text_render_context,
                &mut info.font_context,
                graphics,
                &label,
                text_x,
                text_y,
                item_text_color,
            );

            // Draw submenu indicator below the text if item has submenu
            if has_submenu {
                let arrow_x = item_bounds.x0 + (item_bounds.width() / 2.0); // Center horizontally
                let arrow_y = item_bounds.y1 - 6.0; // Position at bottom with small margin

                // Draw small down arrow below text
                let arrow_size = 2.0;
                let arrow_stroke = Stroke::new(1.0);

                // Simple down arrow (V shape)
                graphics.stroke(
                    &arrow_stroke,
                    Affine::IDENTITY,
                    &Brush::Solid(item_text_color),
                    None,
                    &Line::new(
                        Point::new(arrow_x - arrow_size, arrow_y - arrow_size),
                        Point::new(arrow_x, arrow_y),
                    )
                    .to_path(0.1),
                );
                graphics.stroke(
                    &arrow_stroke,
                    Affine::IDENTITY,
                    &Brush::Solid(item_text_color),
                    None,
                    &Line::new(
                        Point::new(arrow_x, arrow_y),
                        Point::new(arrow_x + arrow_size, arrow_y - arrow_size),
                    )
                    .to_path(0.1),
                );
            }
        }

        // Popup rendering moved to render_postfix for proper z-ordering
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Don't render popup if menu bar not visible
        // Note: If importer is detected and user presses F10, importer_detected is cleared,
        // so we just need to check is_visible()
        if !self.is_visible() {
            return;
        }

        // Render popup menu on top of everything else
        if let Some(open_index) = self.open_menu_index {
            // Calculate position below the menu item
            let item_bounds = self.get_item_bounds(layout, open_index);
            let popup_x = item_bounds.x0;
            let popup_y = item_bounds.y1;

            // Render using unified menu template renderer
            if let Some(ref template) = self.open_template {
                let popup_position = Point::new(popup_x, popup_y);
                let cursor_pos = info.cursor_pos.map(|p| Point::new(p.x, p.y));
                
                render_menu(
                    graphics,
                    template,
                    popup_position,
                    theme,
                    &mut self.text_render_context,
                    &mut info.font_context,
                    cursor_pos,
                    self.hovered_submenu_index,
                );
                
                // Render submenu if open
                if let (Some(submenu_idx), Some(ref submenu_template)) = (self.open_submenu_index, &self.open_submenu_template) {
                    use nptk_core::menu::render::MenuGeometry;
                    let geometry = MenuGeometry::new(template, popup_position, &mut self.text_render_context, &mut info.font_context);
                    let submenu_position = geometry.submenu_origin(submenu_idx);
                    
                    render_menu(
                        graphics,
                        submenu_template,
                        submenu_position,
                        theme,
                        &mut self.text_render_context,
                        &mut info.font_context,
                        cursor_pos,
                        None, // Submenu hover tracking could be added if needed
                    );
                }
            }
        }
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Detect visibility changes and trigger layout update
        let current_visible = self.is_visible();
        if current_visible != self.previous_visible {
            log::info!(
                "MenuBar visibility changed from {} to {}, triggering FORCE layout update",
                self.previous_visible,
                current_visible
            );
            // Use FORCE | LAYOUT | DRAW to ensure the entire layout tree is rebuilt and rendered
            update |= Update::FORCE | Update::LAYOUT | Update::DRAW;
            self.previous_visible = current_visible;
        }

        // Process keyboard events FIRST, especially F10, even if menubar is not visible
        // This allows F10 to show the menubar when it's hidden
        log::debug!(
            "MenuBar update: processing {} keyboard events",
            info.keys.len()
        );
        for (device_id, key_event) in &info.keys {
            log::debug!(
                "MenuBar: key event - device_id={:?}, physical_key={:?}, state={:?}",
                device_id,
                key_event.physical_key,
                key_event.state
            );
            if key_event.state == ElementState::Pressed {
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::F10) => {
                        log::info!(
                            "F10 key detected in MenuBar update() - current visible={}",
                            self.is_visible()
                        );
                        // Toggle menu bar visibility
                        // F10 can override importer detection - if importer is detected but user presses F10,
                        // show the menubar anyway (user wants to see it)
                        let visible = !self.is_visible();
                        self.visible.set(visible);
                        // Persist user's choice to override auto-hide
                        self.user_visibility_override = Some(visible);

                        // If user manually shows the menubar, clear importer detection
                        // so it stays visible until they hide it again or importer queries again
                        if visible {
                            #[cfg(feature = "global-menu")]
                            {
                                log::info!(
                                    "F10 pressed - showing menubar (overriding importer detection)"
                                );
                                self.importer_detected.set(false);
                            }
                        }
                        update |= Update::DRAW | Update::LAYOUT;
                        // Continue processing even if menubar was hidden, in case F10 was pressed
                    },
                    _ => {},
                }
            }
        }

        #[cfg(feature = "global-menu")]
        {
            update |= self.process_global_menu(info);
        }

        // Don't process other events if not visible
        // Note: If importer is detected and user presses F10, importer_detected is cleared,
        // so we just need to check is_visible()
        if !self.is_visible() {
            return update;
        }

        // Get mouse position
        let cursor_pos = info.cursor_pos;

        // Check hover state for main menu items
        let old_hovered = self.hovered_index;
        let old_submenu_hovered = self.hovered_submenu_index;
        self.hovered_index = None;
        self.hovered_submenu_index = None;

        if let Some(pos) = cursor_pos {
            // Check menu templates
            for i in 0..self.menu_templates.len() {
                let item_bounds = self.get_item_bounds(layout, i);
                if pos.x as f64 >= item_bounds.x0
                    && pos.x as f64 <= item_bounds.x1
                    && pos.y as f64 >= item_bounds.y0
                    && pos.y as f64 <= item_bounds.y1
                {
                    self.hovered_index = Some(i);
                    
                    // If a menu is already open and we hover over a different menu item,
                    // switch to that menu (standard GUI behavior)
                    // But only if a menu is already open - don't open on hover
                    if self.open_menu_index.is_some() && self.open_menu_index != Some(i) {
                        // Switch to different menu when another menu is already open
                        self.show_menu_popup(i);
                        update |= Update::DRAW;
                    }
                    // Don't open menu on hover - wait for click
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
                    // Handle unified menu template clicks
                    if hovered < self.menu_templates.len() {
                        // Check if clicking on open menu template popup
                        if let Some(ref template) = self.open_template {
                            let item_bounds = self.get_item_bounds(layout, hovered);
                            let popup_x = item_bounds.x0;
                            let popup_y = item_bounds.y1;
                            let popup_position = Point::new(popup_x, popup_y);
                            
                            // Check if click is in the popup area
                            if let Some(cursor_pos) = cursor_pos {
                                let cursor = Point::new(cursor_pos.x, cursor_pos.y);
                                // Use unified renderer's geometry to check if click is in popup
                                use nptk_core::menu::render::MenuGeometry;
                                let geometry = MenuGeometry::new(template, popup_position, &mut self.text_render_context, &mut info.font_context);
                                
                                if let Some(item_index) = geometry.hit_test_index(cursor) {
                                    if item_index < template.items.len() {
                                        let item = &template.items[item_index];
                                        if item.enabled && !item.is_separator() {
                                            // Route command through MenuManager
                                            if let Some(ref mut manager) = self.menu_manager {
                                                update |= manager.handle_command(item.id);
                                            } else if let Some(ref action) = item.action {
                                                update |= action();
                                            }
                                            
                                            // Close popup after action
                                            self.open_template = None;
                                            self.open_menu_index = None;
                                            update |= Update::DRAW;
                                        }
                                    }
                                } else {
                                    // Click outside popup - close it
                                    self.open_template = None;
                                    self.open_menu_index = None;
                                    update |= Update::DRAW;
                                }
                            }
                        } else {
                            // Click on menu bar item - toggle popup
                            if self.open_menu_index == Some(hovered) {
                                self.open_template = None;
                                self.open_menu_index = None;
                                update |= Update::DRAW;
                            } else {
                                self.show_menu_popup(hovered);
                                update |= Update::DRAW;
                            }
                        }
                        continue;
                    }
                } else {
                    // Click outside - close popup
                    if self.open_template.is_some() {
                        self.open_template = None;
                        self.open_menu_index = None;
                        self.hovered_submenu_index = None;
                        self.open_submenu_index = None;
                        self.open_submenu_template = None;
                        update |= Update::DRAW;
                    }
                }
            }
        }

        // Update popup if open (unified template or legacy)
        if let Some(open_index) = self.open_menu_index {
            // Unified menu template handling
            if let Some(ref template) = self.open_template {
                // Check hover state for unified menu template
                let item_bounds = self.get_item_bounds(layout, open_index);
                let popup_x = item_bounds.x0;
                let popup_y = item_bounds.y1;
                let popup_position = Point::new(popup_x, popup_y);
                
                if let Some(cursor_pos) = cursor_pos {
                    let cursor = Point::new(cursor_pos.x, cursor_pos.y);
                    use nptk_core::menu::render::MenuGeometry;
                    let geometry = MenuGeometry::new(template, popup_position, &mut self.text_render_context, &mut info.font_context);
                    let new_hovered = geometry.hit_test_index(cursor);
                    
                    // Handle submenu opening/closing on hover
                    if new_hovered != self.hovered_submenu_index {
                        // Trigger action callbacks for hover state changes
                        // Only trigger for enabled, non-separator items
                        if let Some(old_idx) = self.hovered_submenu_index {
                            if let Some(old_item) = template.items.get(old_idx) {
                                if old_item.enabled && !old_item.is_separator() {
                                    context.action_callbacks.trigger_leave(old_item.id);
                                }
                            }
                        }
                        if let Some(new_idx) = new_hovered {
                            if let Some(new_item) = template.items.get(new_idx) {
                                if new_item.enabled && !new_item.is_separator() {
                                    context.action_callbacks.trigger_enter(new_item.id);
                                }
                            }
                        }

                        self.hovered_submenu_index = new_hovered;
                        
                        // Check if the hovered item has a submenu
                        if let Some(hovered_idx) = new_hovered {
                            if let Some(item) = template.items.get(hovered_idx) {
                                if item.has_submenu() {
                                    // Open submenu
                                    if let Some(submenu) = item.submenu.clone() {
                                        self.open_submenu_index = Some(hovered_idx);
                                        self.open_submenu_template = Some(submenu);
                                    }
                                } else {
                                    // Close submenu if hovering over non-submenu item
                                    self.open_submenu_index = None;
                                    self.open_submenu_template = None;
                                }
                            }
                        } else {
                            // Cursor left the menu, close submenu
                            self.open_submenu_index = None;
                            self.open_submenu_template = None;
                        }
                        
                        update |= Update::DRAW;
                    }
                }
            }
        }

        // Handle keyboard shortcuts (F10 is handled above before visibility check)
        for (_, key_event) in &info.keys {
            if key_event.state == ElementState::Pressed {
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.open_menu_index.is_some() {
                            self.open_template = None;
                            self.open_menu_index = None;
                            self.hovered_submenu_index = None;
                            self.open_submenu_index = None;
                            self.open_submenu_template = None;
                            update |= Update::DRAW;
                        }
                    },
                    // F10 is handled above before the visibility check
                    PhysicalKey::Code(KeyCode::F10) => {
                        // Already handled above
                    },
                    _ => {},
                }
            }
        }

        update
    }

    fn layout_style(&self) -> StyleNode {
        // Hide the widget by setting display to None when not visible
        // This completely removes it from the layout flow, allowing content below to move up
        // Note: If importer is detected and user presses F10, importer_detected is cleared,
        // so we just need to check is_visible()
        let mut style = self.layout_style.get().clone();
        if !self.is_visible() {
            // Completely remove from layout flow
            style.display = Display::None;
            // Also set size to 0 to ensure no space is taken
            style.size.y = Dimension::length(0.0);
            // Remove all padding and margin
            style.padding.top = LengthPercentage::length(0.0);
            style.padding.bottom = LengthPercentage::length(0.0);
            style.margin.top = LengthPercentageAuto::length(0.0);
            style.margin.bottom = LengthPercentageAuto::length(0.0);
            // Remove min/max size constraints
            style.min_size.y = Dimension::length(0.0);
            style.max_size.y = Dimension::length(0.0);
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

impl MenuBar {
    #[cfg(feature = "global-menu")]
    fn process_global_menu(&mut self, info: &AppInfo) -> Update {
        if !self.global_menu_enabled {
            return Update::empty();
        }

        let mut update = Update::empty();
        let bridge_was_none = self.global_menu_bridge.is_none();
        self.ensure_global_menu_bridge();

        // Poll bridge events to detect importer activity
        if let Some(bridge) = self.global_menu_bridge.as_ref() {
            for event in bridge.poll_events() {
                match event {
                    super::dbus::BridgeEvent::Activated(id) => {
                        if let Some(action) = self.global_menu_actions.get(&id) {
                            update |= action();
                        }
                    },
                    super::dbus::BridgeEvent::ImporterDetected => {
                        // An importer is actively querying the menu - mark as detected
                        if !*self.importer_detected.get() {
                            log::info!("Global menu importer detected - auto-hiding menubar");
                            self.importer_detected.set(true);
                            // Auto-hide the menubar when importer is detected
                            // User can still show it with F10
                            if self.user_visibility_override != Some(true) {
                                self.visible.set(false);
                            }
                            update |= Update::DRAW | Update::LAYOUT;
                        }
                    },
                }
            }
        }

        // Build menu snapshot from unified menu templates
        let (snapshot, actions, signature) = if !self.menu_templates.is_empty() {
            if let Some(ref manager) = self.menu_manager {
                menu_template_to_snapshot(&self.menu_templates, manager)
            } else {
                // No manager, create empty snapshot
                (MenuSnapshot { entries: Vec::new() }, HashMap::new(), 0)
            }
        } else {
            // No templates, empty snapshot
            (MenuSnapshot { entries: Vec::new() }, HashMap::new(), 0)
        };
        let menu_changed = self.global_menu_signature != signature;

        // Always send menu on first bridge initialization or when menu changes
        if bridge_was_none || menu_changed {
            // CRITICAL: Update signature BEFORE registration logic checks it
            if menu_changed {
                self.global_menu_signature = signature;
            } else if bridge_was_none {
                // On first initialization, signature is 0, so update it now
                self.global_menu_signature = signature;
            }
            if let Some(bridge) = self.global_menu_bridge.as_ref() {
                // Send menu immediately, especially on first initialization
                bridge.update_menu(snapshot.clone());
                log::info!(
                    "Menu snapshot sent: {} top-level items",
                    snapshot.entries.len()
                );
            }
        }

        // Register window after menu is available (or updated)
        // On X11/XWayland: use actual X11 window ID
        // On Wayland: use actual Wayland surface ID if available, otherwise dummy ID
        let x11_window_id = current_window_x11_id(info);
        let wayland_surface_id = info.window_wayland_id();
        let window_id = if let Some(x11_id) = x11_window_id {
            // X11/XWayland: use actual X11 window ID
            Some(x11_id as u64)
        } else if let Some(wayland_id) = wayland_surface_id {
            // Wayland: use actual Wayland surface protocol ID
            // This works for both native Wayland and winit-based Wayland windows
            log::debug!(
                "Using Wayland surface ID {} for menu registration",
                wayland_id
            );
            Some(wayland_id as u64)
        } else if is_wayland_session() {
            // Fallback: use dummy window ID 1 if we can't get the surface ID
            // Plasma's compositor will try to discover the menu through app_id matching
            log::debug!("No Wayland surface ID available, using dummy ID 1 for menu registration");
            Some(1u64)
        } else {
            // Fallback: use 0 if we can't determine the window ID
            // This shouldn't happen in normal operation
            log::warn!("Could not determine window ID for menu registration");
            Some(0u64)
        };
        if let Some(bridge) = self.global_menu_bridge.as_ref() {
            // Register window if:
            // 1. Window ID changed, OR
            // 2. Menu was just sent (bridge was just initialized or menu changed)
            let should_register =
                self.last_window_id != window_id || bridge_was_none || menu_changed;
            if should_register && self.global_menu_signature != 0 {
                // Only register if menu has been sent (signature is non-zero)
                bridge.set_window_id(window_id);
                self.last_window_id = window_id;
            } else if window_id.is_some() {
                // Store window ID for later registration once menu is ready
                self.last_window_id = window_id;
            }
        } else {
            self.last_window_id = window_id;
        }

        self.global_menu_actions = actions;

        update
    }

    #[cfg(not(feature = "global-menu"))]
    fn process_global_menu(&mut self, _info: &AppInfo) -> Update {
        Update::empty()
    }

    #[cfg(feature = "global-menu")]
    fn ensure_global_menu_bridge(&mut self) -> Option<&Bridge> {
        if self.global_menu_bridge.is_none() {
            self.global_menu_bridge = Bridge::start();
            if self.global_menu_bridge.is_none() {
                error!("Failed to initialize global menu bridge");
            }
        }
        self.global_menu_bridge.as_ref()
    }
}
