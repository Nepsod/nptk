// SPDX-License-Identifier: LGPL-3.0-only
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Layout, LayoutNode, LayoutStyle, StyleNode, LayoutContext};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::vg::kurbo::Rect;
use nptk_core::vgi::vello_vg::VelloGraphics;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetChildExt, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use std::sync::Arc;
use async_trait::async_trait;

use nptk_widgets::button::Button;
pub use crate::menu_popup::MenuPopup;
use nptk_widgets::text::Text;

use nptk_core::menu::unified::MenuItem as UnifiedMenuItem;
use nptk_core::menu::commands::MenuCommand;

// Re-export UnifiedMenuItem as MenuItem for convenience, or just use UnifiedMenuItem
pub type MenuItem = UnifiedMenuItem;

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
    child: Box<dyn Widget>,
    is_menu_open: Arc<StateSignal<bool>>,
    menu_items: Vec<MenuItem>,
    items_builder: Option<Arc<dyn Fn() -> Vec<MenuItem> + Send + Sync>>,
    on_item_selected: Option<Arc<dyn Fn(String) + Send + Sync>>,
    popup_data: Option<MenuPopup>,
    layout_style: MaybeSignal<LayoutStyle>,
    tooltip: Option<String>,
}

impl std::fmt::Debug for MenuButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuButton")
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
        use nptk_core::layout::{Dimension, LengthPercentage};

        let label_string = label.into();

        // Calculate button width based on text length (similar to MenuPopup)
        let font_size = 16.0;
        // Use chars().count() instead of len() to handle Unicode correctly
        // MenuPopup uses 8.0 pixels per char for 14px font, scaling to 16px font = ~9.1
        let char_count = label_string.chars().count() as f32;
        let estimated_text_width = char_count * 7.5;
        let horizontal_padding = font_size; // Left + right padding
        let button_width = estimated_text_width + horizontal_padding;

        // Text widget has bottom-heavy layout:
        // - Layout height = font_size + line_gap (16 + 7.5 = 23.5px)
        // - Baseline renders at y + font_size (16px from top)
        // - Only line_gap (7.5px) space below baseline
        // To center visually, we need more bottom padding than top
        let bottom_padding = font_size + 2.0; // Compensate for baseline offset

        let text = Text::new(label_string)
            .with_font_size(font_size)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::percent(1.0), // Fill button width
                    Dimension::auto(),
                ),
                ..Default::default()
            });

        let button = Button::new(text)
            .with_style_id("MenuButton")
            .with_invert_text(false) // Don't invert text - transparent background shows dark background
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::length(button_width),
                    Dimension::length(bottom_padding + 4.0),
                ),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(font_size / 2.0),
                    right: LengthPercentage::length(font_size / 2.0),
                    top: LengthPercentage::length(0.0),
                    bottom: LengthPercentage::length(bottom_padding),
                },
                ..Default::default()
            });
        Self {
            child: Box::new(button),
            is_menu_open: Arc::new(StateSignal::new(false)),
            menu_items: Vec::new(),
            items_builder: None,
            on_item_selected: None,
            popup_data: None,
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            tooltip: None,
        }
    }

    /// Create a new menu button with a custom child widget
    pub fn with_child(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            is_menu_open: Arc::new(StateSignal::new(false)),
            menu_items: Vec::new(),
            items_builder: None,
            on_item_selected: None,
            popup_data: None,
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            tooltip: None,
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

    /// Set a builder function to generate menu items dynamically when the menu is opened
    pub fn with_items_builder<F>(mut self, builder: F) -> Self
    where
        F: Fn() -> Vec<MenuItem> + Send + Sync + 'static,
    {
        self.items_builder = Some(Arc::new(builder));
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

    /// Set the tooltip text for the button
    pub fn with_tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
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
        let items = if let Some(ref builder) = self.items_builder {
            builder()
        } else {
            self.menu_items.clone()
        };

        if !items.is_empty() {
            use nptk_core::menu::unified::MenuTemplate;
            
            // We need to clone items to pass to the template
            // And potentially wrap actions to handle on_item_selected
            
            let wrapped_items: Vec<UnifiedMenuItem> = items.iter().map(|item| {
                let mut new_item = item.clone();
                
                // If on_item_selected is set, wrap the action
                if let Some(ref on_selected) = self.on_item_selected {
                    let on_selected = on_selected.clone();
                    let original_action = item.action.clone();
                    let label = item.label.clone();
                    
                    new_item = new_item.with_action(move || {
                        if let Some(ref action) = original_action {
                             let _ = action();
                        }
                        on_selected(label.clone());
                        Update::FORCE
                    });
                } else {
                    // Even without on_item_selected, we ensure the action returns FORCE to close menu
                    // if it wasn't already wrapped (UnifiedMenuItem default action doesn't necessarily close menu?)
                    // ContextMenuState handles closing if `FORCE` is returned? 
                    // Wait, `UnifiedMenuItem` action returns `Update`.
                    // `MenuPopup` logic (in `nptk-widgets-extra` or elsewhere) handles the `Update`.
                    // In `MenuButton::update` line 389: `if popup_update.contains(Update::FORCE) { self.close_menu(); }`
                    // So yes, the action MUST return `Update::FORCE` to close the menu.
                    
                    let original_action = item.action.clone();
                    new_item = new_item.with_action(move || {
                        if let Some(ref action) = original_action {
                            let u = action();
                            if u.contains(Update::FORCE) {
                                return u;
                            }
                            return u | Update::FORCE;
                        }
                        Update::FORCE
                    });
                }
                new_item
            }).collect();

            let template = MenuTemplate::from_items("menu_button", wrapped_items);
            let menu_popup = MenuPopup::new(template);
            
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

#[async_trait(?Send)]
impl Widget for MenuButton {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Render the child button - child layout coordinates are already in screen space
        if !layout.children.is_empty() {
            let mut child_scene = nptk_core::vg::Scene::new();
            let mut child_graphics = VelloGraphics::new(&mut child_scene);
            self.child.render(
                &mut child_graphics,
                &layout.children[0],
                info,
                context.clone(),
            );
            // Append without translation - child layout coordinates are already in screen space
            graphics.append(&child_scene, None);
        }
        // Popup rendering moved to render_postfix for proper z-ordering
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Render popup menu on top of everything else
        if *self.is_menu_open.get() {
            if let Some(ref mut popup) = self.popup_data {
                // Calculate popup position - below the button
                let (popup_width, popup_height) = popup.calculate_size();
                let popup_x = layout.layout.location.x as f64;
                let popup_y = (layout.layout.location.y + layout.layout.size.height) as f64;

                // Create a layout node for the popup
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
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.child.layout_style(context)],
            measure_func: None,
        }
    }

    fn tooltip(&self) -> Option<String> {
        self.tooltip.clone()
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let mut was_button_clicked = false;
        let cursor_pos = info.cursor_pos;

        // Check for button clicks BEFORE propagating to child
        // Use parent's layout for hit detection (since that's where the button actually is)
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left {
                if let Some(pos) = cursor_pos {
                    if pos.x as f32 >= layout.layout.location.x
                        && pos.x as f32 <= layout.layout.location.x + layout.layout.size.width
                        && pos.y as f32 >= layout.layout.location.y
                        && pos.y as f32 <= layout.layout.location.y + layout.layout.size.height
                    {
                        if *state == ElementState::Released {
                            was_button_clicked = true;
                        }
                    }
                }
            }
        }

        // Then propagate update to child
        if !layout.children.is_empty() {
            update |= self
                .child
                .update(&layout.children[0], context.clone(), info).await;
        }

        if *self.is_menu_open.get() {
            if let Some(ref mut popup) = self.popup_data {
                // Calculate popup position - use parent's layout since that's where the button is
                let (popup_width, popup_height) = popup.calculate_size();
                let popup_x = layout.layout.location.x as f64;
                let popup_y = (layout.layout.location.y + layout.layout.size.height) as f64;

                let mut popup_layout = LayoutNode {
                    layout: Layout::default(),
                    children: Vec::new(),
                };
                popup_layout.layout.location.x = popup_x as f32;
                popup_layout.layout.location.y = popup_y as f32;
                popup_layout.layout.size.width = popup_width as f32;
                popup_layout.layout.size.height = popup_height as f32;

                let popup_update = popup.update(&popup_layout, context.clone(), info).await;
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
                    let (popup_width, popup_height) = popup.calculate_size();
                    let popup_rect = Rect::new(
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64 + layout.layout.size.height as f64,
                        layout.layout.location.x as f64 + popup_width,
                        layout.layout.location.y as f64
                            + layout.layout.size.height as f64
                            + popup_height,
                    );
                    let button_rect = Rect::new(
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64,
                        layout.layout.location.x as f64 + layout.layout.size.width as f64,
                        layout.layout.location.y as f64 + layout.layout.size.height as f64,
                    );

                    for (_, button, state) in &info.buttons {
                        if *button == MouseButton::Left && *state == ElementState::Pressed {
                            if !popup_rect.contains((pos.x, pos.y))
                                && !button_rect.contains((pos.x, pos.y))
                            {
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

}
