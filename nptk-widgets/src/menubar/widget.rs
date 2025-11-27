use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
// Overlay system removed for now - using direct rendering instead
#[cfg(feature = "global-menu")]
use super::dbus::{Bridge, BridgeEvent, MenuSnapshot, RemoteMenuNode};
use crate::menu_popup::{MenuBarItem as MenuBarItemImpl, MenuPopup};
#[cfg(feature = "global-menu")]
use log::error;
#[cfg(feature = "global-menu")]
use super::common::platform;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, Display, Layout, LayoutNode, LayoutStyle, LengthPercentage, LengthPercentageAuto, StyleNode};
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
    items: Vec<MenuBarItemImpl>,
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

#[cfg(feature = "global-menu")]
fn build_menu_snapshot(
    items: &[MenuBarItemImpl],
) -> (
    MenuSnapshot,
    HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>,
    u64,
) {
    struct SnapshotBuilder {
        hasher: DefaultHasher,
        next_id: i32,
        nodes: Vec<RemoteMenuNode>,
        actions: HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>,
    }

    impl SnapshotBuilder {
        fn new() -> Self {
            Self {
                hasher: DefaultHasher::new(),
                next_id: 1,
                nodes: Vec::new(),
                actions: HashMap::new(),
            }
        }

        fn build(
            mut self,
            items: &[MenuBarItemImpl],
        ) -> (
            MenuSnapshot,
            HashMap<i32, Arc<dyn Fn() -> Update + Send + Sync>>,
            u64,
        ) {
            self.nodes = self.convert_items(items);
            let signature = self.hasher.finish();
            (
                MenuSnapshot {
                    entries: self.nodes,
                },
                self.actions,
                signature,
            )
        }

        fn convert_items(&mut self, items: &[MenuBarItemImpl]) -> Vec<RemoteMenuNode> {
            items.iter().map(|item| self.convert_item(item)).collect()
        }

        fn convert_item(&mut self, item: &MenuBarItemImpl) -> RemoteMenuNode {
            let is_separator = item.label.trim() == "---";
            item.label.hash(&mut self.hasher);
            item.enabled.hash(&mut self.hasher);
            is_separator.hash(&mut self.hasher);
            item.shortcut.hash(&mut self.hasher);
            self.hasher.write_u64(item.submenu.len() as u64);

            let id = self.next_id;
            self.next_id += 1;

            let children = self.convert_items(&item.submenu);

            if item.submenu.is_empty() && !is_separator {
                if let Some(callback) = item.on_activate.clone() {
                    self.actions.insert(id, callback);
                }
            }

            RemoteMenuNode {
                id,
                label: item.label.clone(),
                enabled: item.enabled && !is_separator,
                is_separator,
                shortcut: item.shortcut.clone(),
                children,
            }
        }
    }

    SnapshotBuilder::new().build(items)
}

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
            popup_data: None,
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
        if let Some(item) = self.get_item_by_index(menu_index).cloned() {
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

    fn get_item_bounds(&self, layout: &LayoutNode, item_index: usize) -> Rect {
        // Calculate precise item width based on actual text metrics
        let font_size = 14.0;
        let horizontal_padding = 12.0; // Left + right padding per item
        let min_width = 40.0; // Minimum width for very short text

        let mut current_x = layout.layout.location.x as f64 + 2.0; // Start with minimal left margin

        #[allow(unused_variables)]
        let items = &self.items;

        // Calculate x position by summing widths of previous items
        for item in items.iter().take(item_index) {
            let text_width = item.label.len() as f64 * (font_size * 0.6);
            let item_width = (text_width + horizontal_padding).max(min_width);
            current_x += item_width;
        }

        // Calculate this item's width with precise text measurement
        let item_width = items
            .get(item_index)
            .map(|item| {
                let text_width = item.label.len() as f64 * (font_size * 0.6);
                (text_width + horizontal_padding).max(min_width)
            })
            .unwrap_or(min_width);

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
            Self::render_text(
                &mut self.text_render_context,
                &mut info.font_context,
                graphics,
                &item.label,
                text_x,
                text_y,
                item_text_color,
            );

            // Draw submenu indicator below the text if item has submenu
            if item.has_submenu() {
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

                // Render the popup
                popup.render(graphics, theme, &popup_layout, info, _context);
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Detect visibility changes and trigger layout update
        let current_visible = self.is_visible();
        if current_visible != self.previous_visible {
            log::info!("MenuBar visibility changed from {} to {}, triggering FORCE layout update", self.previous_visible, current_visible);
            // Use FORCE | LAYOUT | DRAW to ensure the entire layout tree is rebuilt and rendered
            update |= Update::FORCE | Update::LAYOUT | Update::DRAW;
            self.previous_visible = current_visible;
        }

        // Process keyboard events FIRST, especially F10, even if menubar is not visible
        // This allows F10 to show the menubar when it's hidden
        log::debug!("MenuBar update: processing {} keyboard events", info.keys.len());
        for (device_id, key_event) in &info.keys {
            log::debug!("MenuBar: key event - device_id={:?}, physical_key={:?}, state={:?}", device_id, key_event.physical_key, key_event.state);
            if key_event.state == ElementState::Pressed {
                match key_event.physical_key {
                    PhysicalKey::Code(KeyCode::F10) => {
                        log::info!("F10 key detected in MenuBar update() - current visible={}", self.is_visible());
                        // Toggle menu bar visibility
                        // F10 can override importer detection - if importer is detected but user presses F10,
                        // show the menubar anyway (user wants to see it)
                        let visible = !self.is_visible();
                        self.visible.set(visible);
                        // If user manually shows the menubar, clear importer detection
                        // so it stays visible until they hide it again or importer queries again
                        if visible {
                            #[cfg(feature = "global-menu")]
                            {
                                log::info!("F10 pressed - showing menubar (overriding importer detection)");
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
                        if let Some(item) = self.get_item_by_index(i) {
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
                    if let Some(item) = self.get_item_by_index(hovered) {
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
                            } else if let Some(ref callback) = item.on_activate {
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

        // Handle keyboard shortcuts (F10 is handled above before visibility check)
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
    fn get_item_by_index(&self, index: usize) -> Option<&MenuBarItemImpl> {
        self.items.get(index)
    }

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
                            self.visible.set(false);
                            update |= Update::DRAW | Update::LAYOUT;
                        }
                    },
                }
            }
        }

        // Build menu snapshot first to ensure it's available before window registration
        let (snapshot, actions, signature) = build_menu_snapshot(&self.items);
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
                log::info!("Menu snapshot sent: {} top-level items", snapshot.entries.len());
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
            log::debug!("Using Wayland surface ID {} for menu registration", wayland_id);
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
            let should_register = self.last_window_id != window_id || bridge_was_none || menu_changed;
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
