use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
// Overlay system removed for now - using direct rendering instead
#[cfg(feature = "global-menu")]
use self::global_menu::{
    GlobalMenuCommand, GlobalMenuCommandSender, GlobalMenuEvent, GlobalMenuSnapshot,
    GlobalMenuState, RemoteMenuEntry,
};
use crate::menu_popup::{MenuBarItem as MenuBarItemImpl, MenuPopup};
#[cfg(feature = "global-menu")]
use log::error;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, Layout, LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
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

// Re-export MenuBarItem for external use
pub use crate::menu_popup::MenuBarItem;

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
    global_menu_items: Vec<MenuBarItemImpl>,
    #[cfg(feature = "global-menu")]
    global_menu_state: Option<GlobalMenuState>,
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
            #[cfg(feature = "global-menu")]
            global_menu_items: Vec::new(),
            #[cfg(feature = "global-menu")]
            global_menu_state: None,
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
        self.enable_global_menu();
        self
    }

    #[cfg(feature = "global-menu")]
    /// Lazily start the background bridge that mirrors DBus menus into this menu bar.
    pub fn enable_global_menu(&mut self) {
        if self.global_menu_state.is_none() {
            match GlobalMenuState::start() {
                Some(state) => {
                    self.global_menu_state = Some(state);
                },
                None => {
                    error!("Failed to initialize global menu bridge");
                },
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
        let items = self.combined_items();

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

        // Don't process events if not visible
        if !self.is_visible() {
            return update;
        }

        #[cfg(feature = "global-menu")]
        {
            if self.poll_global_menu_events() {
                update |= Update::DRAW;
            }
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
            for i in 0..self.total_items() {
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
                    },
                    PhysicalKey::Code(KeyCode::F10) => {
                        // Toggle menu bar visibility
                        let visible = !self.is_visible();
                        self.visible.set(visible);
                        update |= Update::DRAW | Update::LAYOUT;
                    },
                    _ => {},
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

impl MenuBar {
    #[cfg(feature = "global-menu")]
    fn poll_global_menu_events(&mut self) -> bool {
        let mut changed = false;
        if let Some(state) = self.global_menu_state.as_mut() {
            for event in state.drain_events() {
                match event {
                    GlobalMenuEvent::Snapshot(snapshot) => {
                        if self.rebuild_global_menu_items(snapshot) {
                            self.reset_menu_state();
                            changed = true;
                        }
                    },
                    GlobalMenuEvent::Clear => {
                        if !self.global_menu_items.is_empty() {
                            self.global_menu_items.clear();
                            self.reset_menu_state();
                            changed = true;
                        }
                    },
                }
            }
        }
        changed
    }

    #[cfg(feature = "global-menu")]
    fn rebuild_global_menu_items(&mut self, snapshot: GlobalMenuSnapshot) -> bool {
        let Some(state) = self.global_menu_state.as_ref() else {
            return false;
        };
        let sender = state.command_sender();
        let mut items = Vec::with_capacity(snapshot.entries.len());
        for entry in &snapshot.entries {
            items.push(self.build_remote_item(
                entry,
                &snapshot.address,
                &snapshot.menu_path,
                &sender,
            ));
        }
        self.global_menu_items = items;
        true
    }

    #[cfg(feature = "global-menu")]
    fn build_remote_item(
        &self,
        entry: &RemoteMenuEntry,
        address: &str,
        menu_path: &str,
        sender: &GlobalMenuCommandSender,
    ) -> MenuBarItemImpl {
        let mut item = MenuBarItemImpl {
            id: format!("remote-{address}-{}", entry.id),
            label: if entry.is_separator {
                String::from("---")
            } else {
                entry.label.clone()
            },
            shortcut: entry.shortcut.clone(),
            enabled: entry.enabled && !entry.is_separator,
            submenu: entry
                .submenu
                .iter()
                .map(|child| self.build_remote_item(child, address, menu_path, sender))
                .collect(),
            on_activate: None,
        };

        if !item.enabled {
            return item;
        }

        if item.submenu.is_empty() {
            let action = global_menu::GlobalMenuAction {
                address: address.to_string(),
                menu_path: menu_path.to_string(),
                item_id: entry.id,
            };
            let sender = sender.clone();
            item = item.with_on_activate(move || {
                let _ = sender.send(GlobalMenuCommand::Activate(action.clone()));
                Update::empty()
            });
        }

        item
    }

    fn total_items(&self) -> usize {
        #[cfg(feature = "global-menu")]
        {
            return self.global_menu_items.len() + self.items.len();
        }
        #[cfg(not(feature = "global-menu"))]
        {
            self.items.len()
        }
    }

    fn get_item_by_index(&self, index: usize) -> Option<&MenuBarItemImpl> {
        #[cfg(feature = "global-menu")]
        {
            if index < self.global_menu_items.len() {
                return self.global_menu_items.get(index);
            }
            return self.items.get(index - self.global_menu_items.len());
        }
        #[cfg(not(feature = "global-menu"))]
        {
            self.items.get(index)
        }
    }

    fn combined_items(&self) -> Vec<&MenuBarItemImpl> {
        let mut items = Vec::with_capacity(self.total_items());
        #[cfg(feature = "global-menu")]
        {
            for item in &self.global_menu_items {
                items.push(item);
            }
        }
        for item in &self.items {
            items.push(item);
        }
        items
    }

    #[cfg(feature = "global-menu")]
    fn reset_menu_state(&mut self) {
        self.popup_data = None;
        self.open_menu_index = None;
        self.hovered_index = None;
        self.hovered_submenu_index = None;
    }
}

#[cfg(feature = "global-menu")]
mod global_menu {
    use log::error;
    use std::collections::HashMap;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::thread;
    use system_tray::client::{Client, Event as TrayEvent, UpdateEvent};
    use system_tray::menu::{MenuItem as TrayMenuItem, MenuType, TrayMenu};
    use tokio::runtime::Builder;
    use tokio::sync::mpsc as async_mpsc;

    pub(super) type GlobalMenuCommandSender = async_mpsc::UnboundedSender<GlobalMenuCommand>;

    pub(super) struct GlobalMenuState {
        event_rx: Receiver<GlobalMenuEvent>,
        command_tx: GlobalMenuCommandSender,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl GlobalMenuState {
        pub fn start() -> Option<Self> {
            let (event_tx, event_rx) = mpsc::channel();
            let (command_tx, command_rx) = async_mpsc::unbounded_channel();

            let handle = thread::Builder::new()
                .name("nptk-global-menu".into())
                .spawn(move || {
                    let runtime = Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .expect("global menu runtime");
                    runtime.block_on(async move {
                        run_bridge(event_tx, command_rx).await;
                    });
                })
                .ok()?;

            Some(Self {
                event_rx,
                command_tx,
                thread: Some(handle),
            })
        }

        pub fn drain_events(&mut self) -> Vec<GlobalMenuEvent> {
            let mut events = Vec::new();
            while let Ok(event) = self.event_rx.try_recv() {
                events.push(event);
            }
            events
        }

        pub fn command_sender(&self) -> GlobalMenuCommandSender {
            self.command_tx.clone()
        }
    }

    impl Drop for GlobalMenuState {
        fn drop(&mut self) {
            let _ = self.command_tx.send(GlobalMenuCommand::Shutdown);
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }

    #[derive(Clone, PartialEq)]
    pub(super) struct RemoteMenuEntry {
        pub id: i32,
        pub label: String,
        pub enabled: bool,
        pub shortcut: Option<String>,
        pub submenu: Vec<RemoteMenuEntry>,
        pub is_separator: bool,
    }

    pub(super) struct GlobalMenuSnapshot {
        pub address: String,
        pub menu_path: String,
        pub entries: Vec<RemoteMenuEntry>,
    }

    pub(super) enum GlobalMenuEvent {
        Snapshot(GlobalMenuSnapshot),
        Clear,
    }

    #[derive(Clone)]
    pub(super) struct GlobalMenuAction {
        pub address: String,
        pub menu_path: String,
        pub item_id: i32,
    }

    #[derive(Clone)]
    pub(super) enum GlobalMenuCommand {
        Activate(GlobalMenuAction),
        Shutdown,
    }

    #[derive(Clone)]
    struct RemoteAppState {
        menu_path: Option<String>,
        last_entries: Option<Vec<RemoteMenuEntry>>,
    }

    impl RemoteAppState {
        fn new() -> Self {
            Self {
                menu_path: None,
                last_entries: None,
            }
        }
    }

    async fn run_bridge(
        event_tx: Sender<GlobalMenuEvent>,
        mut command_rx: async_mpsc::UnboundedReceiver<GlobalMenuCommand>,
    ) {
        let client = match Client::new().await {
            Ok(client) => client,
            Err(err) => {
                error!("Failed to start StatusNotifier client: {err}");
                let _ = event_tx.send(GlobalMenuEvent::Clear);
                return;
            },
        };

        let mut updates = client.subscribe();
        let mut apps: HashMap<String, RemoteAppState> = HashMap::new();
        let mut active: Option<String> = None;

        loop {
            tokio::select! {
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        GlobalMenuCommand::Activate(action) => {
                            if let Err(err) = client
                                .activate(system_tray::client::ActivateRequest::MenuItem {
                                    address: action.address.clone(),
                                    menu_path: action.menu_path.clone(),
                                    submenu_id: action.item_id,
                                })
                                .await
                            {
                                error!("Failed to activate global menu item: {err:?}");
                            }
                        }
                        GlobalMenuCommand::Shutdown => break,
                    }
                }
                Ok(event) = updates.recv() => {
                    match event {
                        TrayEvent::Add(address, _item) => {
                            apps.entry(address).or_insert_with(RemoteAppState::new);
                        }
                        TrayEvent::Update(address, update) => {
                            let entry = apps.entry(address.clone()).or_insert_with(RemoteAppState::new);
                            match update {
                                UpdateEvent::MenuConnect(path) => {
                                    entry.menu_path = Some(path);
                                }
                                UpdateEvent::Menu(menu) => {
                                    if let Some(snapshot) = build_snapshot(&address, entry, menu) {
                                        active = Some(address.clone());
                                        let _ = event_tx.send(GlobalMenuEvent::Snapshot(snapshot));
                                    }
                                }
                                _ => {}
                            }
                        }
                        TrayEvent::Remove(address) => {
                            let removed_active = active.as_deref() == Some(address.as_str());
                            apps.remove(&address);
                            if removed_active {
                                active = promote_next(&apps, &event_tx);
                            }
                        }
                    }
                }
                else => break,
            }
        }

        let _ = event_tx.send(GlobalMenuEvent::Clear);
    }

    fn build_snapshot(
        address: &str,
        entry: &mut RemoteAppState,
        menu: TrayMenu,
    ) -> Option<GlobalMenuSnapshot> {
        let menu_path = entry.menu_path.clone()?;
        let entries = convert_entries(&menu.submenus);
        entry.last_entries = Some(entries.clone());
        Some(GlobalMenuSnapshot {
            address: address.to_string(),
            menu_path,
            entries,
        })
    }

    fn promote_next(
        apps: &HashMap<String, RemoteAppState>,
        event_tx: &Sender<GlobalMenuEvent>,
    ) -> Option<String> {
        for (address, state) in apps {
            if let (Some(path), Some(entries)) = (&state.menu_path, &state.last_entries) {
                let snapshot = GlobalMenuSnapshot {
                    address: address.clone(),
                    menu_path: path.clone(),
                    entries: entries.clone(),
                };
                let _ = event_tx.send(GlobalMenuEvent::Snapshot(snapshot));
                return Some(address.clone());
            }
        }
        let _ = event_tx.send(GlobalMenuEvent::Clear);
        None
    }

    fn convert_entries(items: &[TrayMenuItem]) -> Vec<RemoteMenuEntry> {
        items
            .iter()
            .filter(|item| item.visible)
            .map(|item| RemoteMenuEntry {
                id: item.id,
                label: item.label.clone().unwrap_or_else(|| {
                    String::from(if matches!(item.menu_type, MenuType::Separator) {
                        "---"
                    } else {
                        ""
                    })
                }),
                enabled: item.enabled,
                shortcut: format_shortcut(item),
                submenu: convert_entries(&item.submenu),
                is_separator: matches!(item.menu_type, MenuType::Separator),
            })
            .collect()
    }

    fn format_shortcut(item: &TrayMenuItem) -> Option<String> {
        let shortcuts = item.shortcut.as_ref()?;
        if shortcuts.is_empty() {
            return None;
        }

        let combos: Vec<String> = shortcuts
            .iter()
            .map(|combo| {
                combo
                    .iter()
                    .map(|key| format_key(key))
                    .collect::<Vec<_>>()
                    .join("+")
            })
            .collect();

        Some(combos.join(", "))
    }

    fn format_key(key: &str) -> &str {
        match key {
            "Control" => "Ctrl",
            "Super" => "Cmd",
            other => other,
        }
    }
}
