use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Layout, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{MaybeSignal, state::StateSignal, Signal};
use std::sync::Arc;
use nptk_core::vg::kurbo::Rect;
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt, WidgetChildExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

pub use crate::menu_popup::{MenuBarItem, MenuPopup};
use crate::button::Button;
use crate::text::Text;

/// Represents a menu item in a popup menu
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MenuItem {
    /// A menu item with a label and optional keyboard shortcut
    Item(String, Option<String>),
    /// A separator line between menu items
    Separator,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(_id: impl ToString, label: impl ToString) -> Self {
        Self::Item(label.to_string(), None)
    }

    /// Set the keyboard shortcut for this item
    pub fn with_shortcut(mut self, shortcut: impl ToString) -> Self {
        if let Self::Item(_, ref mut s) = self {
            *s = Some(shortcut.to_string());
        }
        self
    }

    /// Set whether this item is enabled
    pub fn with_enabled(self, _enabled: bool) -> Self {
        if let Self::Item(_, _) = self {
            // No-op for now, as enabled state is not directly reflected in MenuItem
        }
        self
    }

    /// Set the callback for when this item is activated
    pub fn with_on_activate<F>(self, _callback: F) -> Self 
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        if let Self::Item(_, _) = self {
            // No-op for now, as on_activate is not directly reflected in MenuItem
        }
        self
    }
}

/// A button that displays a popup menu when clicked
///
/// ### Theming
/// Styling the menu button requires the following properties:
/// - `color_pressed` - The color of the button when pressed.
/// - `color_idle` - The color of the button when not pressed and not hovered (idling).
/// - `color_hovered` - The color of the button when hovered on.
/// - `color_focused` - The color of the button when focused (optional).
/// - `menu_background` - The background color of the popup menu.
/// - `menu_border` - The border color of the popup menu.
/// - `menu_text` - The text color for menu items.
/// - `menu_hovered` - The background color when hovering over menu items.
/// - `menu_disabled` - The text color for disabled menu items.
pub struct MenuButton {
    widget_id: WidgetId,
    child: Box<dyn Widget>,
    is_menu_open: Arc<StateSignal<bool>>,
    menu_items: Vec<MenuItem>,
    on_item_selected: Option<Arc<dyn Fn(String) + Send + Sync>>,
    popup_data: Option<MenuPopup>,
    layout_style: MaybeSignal<LayoutStyle>,
}

impl std::fmt::Debug for MenuButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuButton")
            .field("widget_id", &self.widget_id)
            .field("is_menu_open", &*self.is_menu_open.get())
            .field("menu_items", &self.menu_items.len())
            .field("popup_data", &self.popup_data.is_some())
            .field("layout_style", &*self.layout_style.get())
            .finish()
    }
}

impl MenuButton {
    /// Create a new menu button with the given label
    pub fn new(label: impl Into<String>) -> Self {
        let button = Button::new(Text::new(label.into()));
        Self {
            widget_id: WidgetId::new("nptk_widgets", "MenuButton"),
            child: Box::new(button),
            is_menu_open: Arc::new(StateSignal::new(false)),
            menu_items: Vec::new(),
            on_item_selected: None,
            popup_data: None,
            layout_style: MaybeSignal::value(LayoutStyle::default()),
        }
    }
    
    /// Create a new menu button with a custom child widget
    pub fn with_child(child: impl Widget + 'static) -> Self {
        Self {
            widget_id: WidgetId::new("nptk_widgets", "MenuButton"),
            child: Box::new(child),
            is_menu_open: Arc::new(StateSignal::new(false)),
            menu_items: Vec::new(),
            on_item_selected: None,
            popup_data: None,
            layout_style: MaybeSignal::value(LayoutStyle::default()),
        }
    }

    /// Add a menu item to the popup menu
    pub fn with_menu_item(mut self, item: MenuItem) -> Self {
        self.menu_items.push(item);
        self
    }

    /// Add multiple menu items to the popup menu
    pub fn with_menu_items(mut self, items: Vec<MenuItem>) -> Self {
        self.menu_items = items;
        self
    }

    /// Set the callback for when an item is selected from the popup menu
    pub fn with_on_item_selected<F>(mut self, callback: F) -> Self 
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_item_selected = Some(Arc::new(callback));
        self
    }

    /// Check if the menu is currently open
    pub fn is_menu_open(&self) -> bool {
        *self.is_menu_open.get()
    }

    /// Open the popup menu
    pub fn open_menu(&self) {
        self.is_menu_open.set(true);
    }

    /// Close the popup menu
    pub fn close_menu(&mut self) {
        self.is_menu_open.set(false);
        // self.popup_data = None; // Temporarily disabled
    }

    /// Toggle the popup menu
    pub fn toggle_menu(&self) {
        self.is_menu_open.set(!self.is_menu_open());
    }

    /// Create and show the menu popup
    fn show_menu_popup(&mut self, _layout: &LayoutNode, _info: &mut AppInfo) {
        if !self.menu_items.is_empty() {
            let menu_bar_items: Vec<MenuBarItem> = self
                .menu_items
                .iter()
                .filter_map(|item| match item {
                    MenuItem::Item(label, shortcut) => Some(MenuBarItem::new(label.clone(), label.clone())
                        .with_shortcut(shortcut.clone().unwrap_or_default())
                        .with_enabled(true)),
                    MenuItem::Separator => None, // Skip separators for now
                })
                .collect();

            let mut menu_popup = MenuPopup::new().with_items(menu_bar_items);
            let menu_items_clone = self.menu_items.clone();
            let on_item_selected_clone = self.on_item_selected.clone();

            // Always set an on_item_selected callback to handle closing
            menu_popup = menu_popup.with_on_item_selected(Box::new(move |index| {
                // Call user callback if provided
                if let Some(ref on_item_selected) = on_item_selected_clone {
                    if let Some(MenuItem::Item(label, _)) = menu_items_clone.get(index) {
                        on_item_selected(label.clone());
                    }
                }
                // Return FORCE to signal that an item was selected and menu should close
                Update::FORCE
            }));
            
            // Add a callback to close the menu when an item is selected or closed
            // Note: We'll handle closing in the update method instead of using a callback
            // to avoid Send/Sync issues with StateSignal

            self.popup_data = Some(menu_popup);
        }
    }

}

impl WidgetChildExt for MenuButton {
    fn set_child(&mut self, child: impl Widget + 'static) {
        self.child = Box::new(child);
    }
}

impl WidgetLayoutExt for MenuButton {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for MenuButton {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Render the child button first
        self.child.render(scene, theme, &layout.children[0], info, context.clone());

        // Then, if the menu is open, render the popup on top
        if *self.is_menu_open.get() {
            if let Some(ref mut popup) = self.popup_data {
                // Calculate popup position based on the child button's layout
                let child_layout = &layout.children[0];
                let button_bounds = Rect::new(
                    child_layout.layout.location.x as f64,
                    child_layout.layout.location.y as f64,
                    (child_layout.layout.location.x + child_layout.layout.size.width) as f64,
                    (child_layout.layout.location.y + child_layout.layout.size.height) as f64,
                );

                let (popup_width, popup_height) = popup.calculate_size();
                let popup_x = button_bounds.x0;
                let popup_y = button_bounds.y1;

                // Create a layout node for the popup
                let mut popup_layout = LayoutNode {
                    layout: Layout::default(),
                    children: Vec::new(),
                };
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;

                popup.render(scene, theme, &popup_layout, info, context);
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.child.layout_style()],
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let mut was_button_clicked = false;
        let cursor_pos = info.cursor_pos;

        // Check for button clicks BEFORE propagating to child
        // Use the child button's layout bounds for hit detection
        if !layout.children.is_empty() {
            let child_layout = &layout.children[0];
            for (_, button, state) in &info.buttons {
                if *button == MouseButton::Left {
                    if let Some(pos) = cursor_pos {
                        let bounds = child_layout.layout.size;
                        let location = child_layout.layout.location;
                        if pos.x as f32 >= location.x
                            && pos.x as f32 <= location.x + bounds.width
                            && pos.y as f32 >= location.y
                            && pos.y as f32 <= location.y + bounds.height
                        {
                            if *state == ElementState::Released {
                                was_button_clicked = true;
                            }
                        }
                    }
                }
            }
        }

        // Then propagate update to child
        update |= self.child.update(&layout.children[0], context.clone(), info);

        if *self.is_menu_open.get() {
            if let Some(ref mut popup) = self.popup_data {
                // Calculate popup position for update using child button's layout
                let child_layout = &layout.children[0];
                let button_bounds = Rect::new(
                    child_layout.layout.location.x as f64,
                    child_layout.layout.location.y as f64,
                    (child_layout.layout.location.x + child_layout.layout.size.width) as f64,
                    (child_layout.layout.location.y + child_layout.layout.size.height) as f64,
                );

                let (popup_width, popup_height) = popup.calculate_size();
                let popup_x = button_bounds.x0;
                let popup_y = button_bounds.y1;

                let mut popup_layout = LayoutNode {
                    layout: Layout::default(),
                    children: Vec::new(),
                };
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;

                let popup_update = popup.update(&popup_layout, context.clone(), info);
                update |= popup_update;
                
                // If the popup returned FORCE, it means an item was selected - close the menu
                if popup_update.contains(Update::FORCE) {
                    self.close_menu();
                }
            }
            
            // Handle click-outside-to-close
            let mut click_outside = false;
            if let Some(pos) = cursor_pos {
                if let Some(ref popup) = self.popup_data {
                    let child_layout = &layout.children[0];
                    let (popup_width, popup_height) = popup.calculate_size();
                    let popup_rect = Rect::new(
                        child_layout.layout.location.x as f64,
                        child_layout.layout.location.y as f64 + child_layout.layout.size.height as f64,
                        child_layout.layout.location.x as f64 + popup_width,
                        child_layout.layout.location.y as f64 + child_layout.layout.size.height as f64 + popup_height,
                    );
                    let button_rect = Rect::new(
                        child_layout.layout.location.x as f64,
                        child_layout.layout.location.y as f64,
                        child_layout.layout.location.x as f64 + child_layout.layout.size.width as f64,
                        child_layout.layout.location.y as f64 + child_layout.layout.size.height as f64,
                    );

                    for (_, button, state) in &info.buttons {
                        if *button == MouseButton::Left && *state == ElementState::Pressed {
                            if !popup_rect.contains((pos.x, pos.y)) && !button_rect.contains((pos.x, pos.y)) {
                                click_outside = true;
                            }
                        }
                    }
                }
            }

            if was_button_clicked || click_outside {
                self.close_menu();
                update |= Update::DRAW;
            }

        } else if was_button_clicked {
            self.is_menu_open.set(true);
            self.show_menu_popup(layout, info);
            update |= Update::DRAW;
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        self.widget_id.clone()
    }
}
