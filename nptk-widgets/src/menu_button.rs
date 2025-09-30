use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusId, FocusState, FocusableWidget, FocusProperties, FocusBounds};
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Vec2, Stroke, Line, Point};
use nptk_core::vg::peniko::{Brush, Fill, Color};
use nptk_core::vg::Scene;
use nptk_core::text_render::TextRenderContext;
use nptk_core::widget::{BoxedWidget, Widget, WidgetChildExt, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton, KeyCode, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::sync::Arc;

/// Represents a menu item in the popup menu
#[derive(Clone)]
pub struct MenuItem {
    /// Unique identifier for the menu item
    pub id: String,
    /// Display label for the menu item
    pub label: String,
    /// Optional keyboard shortcut text
    pub shortcut: Option<String>,
    /// Whether the menu item is enabled
    pub enabled: bool,
    /// Callback function called when the item is activated
    pub on_activate: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(id: impl ToString, label: impl ToString) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            shortcut: None,
            enabled: true,
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

    /// Set the callback for when this item is activated
    pub fn with_on_activate<F>(mut self, callback: F) -> Self 
    where
        F: Fn() -> Update + Send + Sync + 'static,
    {
        self.on_activate = Some(Arc::new(callback));
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
    child: BoxedWidget,
    menu_items: Vec<MenuItem>,
    state: ButtonState,
    layout_style: MaybeSignal<LayoutStyle>,
    focus_id: FocusId,
    focus_state: FocusState,
    focus_via_keyboard: bool,
    
    // Menu state
    menu_open: StateSignal<bool>,
    hovered_menu_item: Option<usize>,
    text_render_context: TextRenderContext,
}

impl MenuButton {
    /// Create a new menu button with the given child widget
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            menu_items: Vec::new(),
            state: ButtonState::Idle,
            layout_style: LayoutStyle {
                padding: layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(12.0),
                    right: LengthPercentage::length(12.0),
                    top: LengthPercentage::length(2.0),
                    bottom: LengthPercentage::length(10.0),
                },
                ..Default::default()
            }
            .into(),
            focus_id: FocusId::new(),
            focus_state: FocusState::None,
            focus_via_keyboard: false,
            menu_open: StateSignal::new(false),
            hovered_menu_item: None,
            text_render_context: TextRenderContext::new(),
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

    /// Check if the menu is currently open
    pub fn is_menu_open(&self) -> bool {
        *self.menu_open.get()
    }

    /// Open the popup menu
    pub fn open_menu(&self) {
        self.menu_open.set(true);
    }

    /// Close the popup menu
    pub fn close_menu(&self) {
        self.menu_open.set(false);
    }

    /// Toggle the popup menu
    pub fn toggle_menu(&self) {
        self.menu_open.set(!self.is_menu_open());
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "MenuButton")
    }

    fn get_menu_bounds(&self, layout: &LayoutNode) -> Rect {
        let button_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        let menu_width = 200.0;
        let item_height = 24.0;
        let menu_height = self.menu_items.len() as f64 * item_height;

        // Position menu below the button
        Rect::new(
            button_bounds.x0,
            button_bounds.y1,
            button_bounds.x0 + menu_width,
            button_bounds.y1 + menu_height,
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
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Update focus state from focus manager
        if let Ok(manager) = info.focus_manager.lock() {
            self.focus_state = manager.get_focus_state(self.focus_id);
        }

        let widget_theme = theme.of(self.widget_id());
        
        // Render the button (similar to Button widget)
        let brush = if let Some(ref style) = widget_theme {
            // Check for focused state first (only if focus was via keyboard), then button state
            if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) && self.focus_via_keyboard {
                if let Some(focused_color) = style.get_color("color_focused") {
                    Brush::Solid(focused_color)
                } else {
                    // Fallback to hovered color for focus if no focused color is defined
                    Brush::Solid(style.get_color("color_hovered").unwrap_or(theme.defaults().interactive().hover()))
                }
            } else {
                match self.state {
                    ButtonState::Idle => Brush::Solid(style.get_color("color_idle").unwrap()),
                    ButtonState::Hovered => Brush::Solid(style.get_color("color_hovered").unwrap()),
                    ButtonState::Pressed => Brush::Solid(style.get_color("color_pressed").unwrap()),
                    ButtonState::Released => Brush::Solid(style.get_color("color_hovered").unwrap()),
                }
            }
        } else {
            // Default colors - only show focus color if focus was via keyboard
            if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) && self.focus_via_keyboard {
                Brush::Solid(theme.defaults().interactive().hover())
            } else {
                Brush::Solid(match self.state {
                    ButtonState::Idle => theme.defaults().interactive().inactive(),
                    ButtonState::Hovered => theme.defaults().interactive().hover(),
                    ButtonState::Pressed => theme.defaults().interactive().active(),
                    ButtonState::Released => theme.defaults().interactive().hover(),
                })
            }
        };

        let button_rect = RoundedRect::from_rect(
            Rect::new(
                layout_node.layout.location.x as f64,
                layout_node.layout.location.y as f64,
                (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
            ),
            RoundedRectRadii::from_single_radius(10.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &brush,
            None,
            &button_rect,
        );

        // Draw focus indicator (only if focus was gained via keyboard)
        if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) && self.focus_via_keyboard {
            let focus_brush = Brush::Solid(Color::from_rgb8(100, 150, 255)); // Blue focus border
            
            scene.stroke(
                &Stroke::new(3.0),
                Affine::default(),
                &focus_brush,
                None,
                &button_rect,
            );
        }

        // Render button child
        {
            theme.globals_mut().invert_text_color = true;

            let mut child_scene = Scene::new();

            self.child.render(
                &mut child_scene,
                theme,
                &layout_node.children[0],
                info,
                context,
            );

            scene.append(
                &child_scene,
                Some(Affine::translate(Vec2::new(
                    layout_node.layout.location.x as f64,
                    layout_node.layout.location.y as f64,
                ))),
            );

            theme.globals_mut().invert_text_color = false;
        }

        // Render popup menu if open
        if self.is_menu_open() && !self.menu_items.is_empty() {
            let menu_bounds = self.get_menu_bounds(layout_node);
            
            // Pre-calculate theme colors for menu
            let menu_bg_color = if let Some(ref style) = widget_theme {
                style.get_color("menu_background").unwrap_or(Color::from_rgb8(240, 240, 240))
            } else {
                Color::from_rgb8(240, 240, 240)
            };
            
            let menu_border_color = if let Some(ref style) = widget_theme {
                style.get_color("menu_border").unwrap_or(Color::from_rgb8(200, 200, 200))
            } else {
                Color::from_rgb8(200, 200, 200)
            };
            
            let menu_text_color = if let Some(ref style) = widget_theme {
                style.get_color("menu_text").unwrap_or(Color::BLACK)
            } else {
                Color::BLACK
            };
            
            let menu_hovered_color = if let Some(ref style) = widget_theme {
                style.get_color("menu_hovered").unwrap_or(Color::from_rgb8(220, 220, 220))
            } else {
                Color::from_rgb8(220, 220, 220)
            };
            
            let menu_disabled_color = if let Some(ref style) = widget_theme {
                style.get_color("menu_disabled").unwrap_or(Color::from_rgb8(150, 150, 150))
            } else {
                Color::from_rgb8(150, 150, 150)
            };

            // Draw menu background
            let menu_rounded = RoundedRect::new(
                menu_bounds.x0,
                menu_bounds.y0,
                menu_bounds.x1,
                menu_bounds.y1,
                RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
            );
            scene.fill(Fill::NonZero, Affine::IDENTITY, menu_bg_color, None, &menu_rounded);
            
            // Draw menu border
            let stroke = Stroke::new(1.0);
            scene.stroke(&stroke, Affine::IDENTITY, menu_border_color, None, &menu_rounded);

            // Draw menu items
            let item_height = 24.0;
            for (i, item) in self.menu_items.iter().enumerate() {
                let item_y = menu_bounds.y0 + (i as f64 * item_height);
                let item_rect = Rect::new(
                    menu_bounds.x0,
                    item_y,
                    menu_bounds.x1,
                    item_y + item_height,
                );

                // Determine item colors
                let (item_text_color, item_bg_color) = if !item.enabled {
                    (menu_disabled_color, Color::TRANSPARENT)
                } else if Some(i) == self.hovered_menu_item {
                    (menu_text_color, menu_hovered_color)
                } else {
                    (menu_text_color, Color::TRANSPARENT)
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

                // Draw item text
                if item.label != "---" { // Skip separators
                    let text_x = item_rect.x0 + 8.0;
                    let text_y = item_rect.y0 + 2.0;
                    Self::render_text(&mut self.text_render_context, &mut info.font_context, scene, &item.label, text_x, text_y, item_text_color);
                    
                    // Draw keyboard shortcut if present
                    if let Some(ref shortcut) = item.shortcut {
                        let shortcut_x = item_rect.x1 - 60.0; // Right-aligned
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
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.child.layout_style()],
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let old_state = self.state;
        let old_focus_state = self.focus_state;

        // Register this button with the focus manager
        if let Ok(mut manager) = info.focus_manager.lock() {
            let focusable_widget = FocusableWidget {
                id: self.focus_id,
                properties: FocusProperties {
                    tab_focusable: true,
                    click_focusable: true,
                    tab_index: 0,
                    accepts_keyboard: true,
                },
                bounds: FocusBounds {
                    x: layout.layout.location.x,
                    y: layout.layout.location.y,
                    width: layout.layout.size.width,
                    height: layout.layout.size.height,
                },
            };
            manager.register_widget(focusable_widget);
            
            // Update our focus state
            let new_focus_state = manager.get_focus_state(self.focus_id);
            
            // Track if focus was gained via keyboard using global state
            if matches!(new_focus_state, FocusState::Gained) && !matches!(old_focus_state, FocusState::Focused) {
                // Check if the focus manager indicates this was a keyboard focus change
                self.focus_via_keyboard = manager.was_last_focus_via_keyboard();
            } else if matches!(new_focus_state, FocusState::Lost | FocusState::None) {
                self.focus_via_keyboard = false;
            } else if matches!(new_focus_state, FocusState::Focused) {
                // Keep the existing keyboard focus state if we're staying focused
                // This ensures the border stays visible while navigating with Tab
            }
            
            self.focus_state = new_focus_state;
        }

        // Handle keyboard input when focused
        if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
            for (_, key_event) in &info.keys {
                match key_event.state {
                    ElementState::Pressed => {
                        match key_event.physical_key {
                            PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter) => {
                                // Toggle menu via keyboard
                                self.toggle_menu();
                                update |= Update::DRAW;
                            }
                            PhysicalKey::Code(KeyCode::Escape) => {
                                // Close menu if open
                                if self.is_menu_open() {
                                    self.close_menu();
                                    update |= Update::DRAW;
                                }
                            }
                            _ => {}
                        }
                    }
                    ElementState::Released => {
                        match key_event.physical_key {
                            PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter) => {
                                // Reset button state after keyboard release
                                if self.state == ButtonState::Pressed {
                                    self.state = ButtonState::Idle;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Handle menu item hover and clicks if menu is open
        if self.is_menu_open() {
            let old_hovered = self.hovered_menu_item;
            self.hovered_menu_item = None;
            
            if let Some(cursor_pos) = info.cursor_pos {
                let menu_bounds = self.get_menu_bounds(layout);
                let item_height = 24.0;
                
                for (i, _item) in self.menu_items.iter().enumerate() {
                    let item_y = menu_bounds.y0 + (i as f64 * item_height);
                    let item_rect = Rect::new(
                        menu_bounds.x0,
                        item_y,
                        menu_bounds.x1,
                        item_y + item_height,
                    );
                    
                    if cursor_pos.x as f64 >= item_rect.x0
                        && cursor_pos.x as f64 <= item_rect.x1
                        && cursor_pos.y as f64 >= item_rect.y0
                        && cursor_pos.y as f64 <= item_rect.y1
                    {
                        self.hovered_menu_item = Some(i);
                        break;
                    }
                }
            }
            
            if old_hovered != self.hovered_menu_item {
                update |= Update::DRAW;
            }
            
            // Handle menu item clicks
            for (_, button, state) in &info.buttons {
                if *button == MouseButton::Left && *state == ElementState::Pressed {
                    if let Some(hovered_index) = self.hovered_menu_item {
                        if let Some(item) = self.menu_items.get(hovered_index) {
                            if item.enabled && item.label != "---" {
                                // Execute menu item callback
                                if let Some(ref callback) = item.on_activate {
                                    update |= callback();
                                }
                                self.close_menu();
                                update |= Update::DRAW;
                            }
                        }
                    } else {
                        // Click outside menu - close it
                        self.close_menu();
                        update |= Update::DRAW;
                    }
                }
            }
        }

        // Handle button hover and clicks
        if let Some(cursor) = info.cursor_pos {
            let button_bounds = Rect::new(
                layout.layout.location.x as f64,
                layout.layout.location.y as f64,
                (layout.layout.location.x + layout.layout.size.width) as f64,
                (layout.layout.location.y + layout.layout.size.height) as f64,
            );
            
            if cursor.x as f64 >= button_bounds.x0
                && cursor.x as f64 <= button_bounds.x1
                && cursor.y as f64 >= button_bounds.y0
                && cursor.y as f64 <= button_bounds.y1
            {
                // fixes state going to hover if the button is pressed but not yet released
                if self.state != ButtonState::Pressed {
                    self.state = ButtonState::Hovered;
                }

                // check for click
                for (_, btn, el) in &info.buttons {
                    if *btn == MouseButton::Left {
                        match el {
                            ElementState::Pressed => {
                                self.state = ButtonState::Pressed;
                            },

                            // actually fire the event if the button is released
                            ElementState::Released => {
                                self.state = ButtonState::Released;
                                // Toggle menu on button click
                                self.toggle_menu();
                                update |= Update::DRAW;
                            },
                        }
                    }
                }
            } else {
                // cursor not in area, so button is idle
                self.state = ButtonState::Idle;
            }
        } else {
            // cursor is not in window, so button is idle
            self.state = ButtonState::Idle;
        }

        // update on state change, due to re-coloring
        if old_state != self.state {
            update |= Update::DRAW;
        }

        // update on focus state change, due to re-coloring
        if old_focus_state != self.focus_state {
            update |= Update::DRAW;
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        self.widget_id()
    }
}

/// The internal state of the button.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ButtonState {
    /// The button is idling (inactive).
    Idle,
    /// The cursor is hovering over the button.
    Hovered,
    /// The cursor is hovering over the button and the left click button is pressed.
    Pressed,
    /// The cursor is hovering over the button and the left click button is released.
    /// This is when the `on_pressed` function is called.
    Released,
}
