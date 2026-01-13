use crate::theme_rendering::render_button_with_theme;
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusBounds, FocusId, FocusProperties, FocusState, FocusableWidget};
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Rect, Shape};
use nptk_core::vg::peniko::Mix;
use nptk_core::vgi::{vello_vg::VelloGraphics, Graphics};
use nptk_core::widget::{BoxedWidget, Widget, WidgetChildExt, WidgetLayoutExt};
use nptk_core::window::{ElementState, KeyCode, MouseButton, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;
use std::time::{Duration, Instant};

/// An interactive area with a child widget that runs a closure when pressed.
///
/// ### Theming
/// Styling the button require following properties:
/// - `color_pressed` -  The color of the button when pressed.
/// - `color_idle` - The color of the button when not pressed and not hovered (idling).
/// - `color_hovered` - The color of the button when hovered on.
/// - `color_focused` - The color of the button when focused (optional).
pub struct Button {
    child: BoxedWidget,
    state: ButtonState,
    on_pressed: MaybeSignal<Update>,
    layout_style: MaybeSignal<LayoutStyle>,
    focus_id: FocusId,
    focus_state: FocusState,
    focus_via_keyboard: bool, // Track if focus was gained via keyboard
    disabled: bool,
    // Repeat logic
    repeat_enabled: bool,
    repeat_delay: Duration,
    repeat_interval: Duration,
    press_start_time: Option<Instant>,
    last_repeat_time: Option<Instant>,
    style_id: &'static str,
    // Text color inversion control
    invert_text: bool,
    // Tooltip text
    tooltip: Option<String>,
    // Tooltip state tracking to prevent rapid cycling
    tooltip_hover_state: bool,
    // Status bar tip text (shown in status bar when hovering)
    status_tip: Option<String>,
    // Status tip state tracking to prevent rapid cycling
    status_tip_hover_state: bool,
}

impl Button {
    /// Create a new button with the given child widget.
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Box::new(child),
            state: ButtonState::Idle,
            on_pressed: MaybeSignal::value(Update::empty()),
            layout_style: LayoutStyle {
                padding: layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(12.0),
                    right: LengthPercentage::length(12.0),
                    top: LengthPercentage::length(2.0),
                    bottom: LengthPercentage::length(10.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            }
            .into(),
            focus_id: FocusId::new(),
            focus_state: FocusState::None,
            focus_via_keyboard: false,
            disabled: false,
            repeat_enabled: false,
            repeat_delay: Duration::from_millis(500),
            repeat_interval: Duration::from_millis(100),
            press_start_time: None,
            last_repeat_time: None,
            style_id: "Button",
            invert_text: true, // Default: invert text for colored buttons
            tooltip: None,
            tooltip_hover_state: false,
            status_tip: None,
            status_tip_hover_state: false,
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Sets the function to be called when the button is pressed.
    pub fn with_on_pressed(self, on_pressed: impl Into<MaybeSignal<Update>>) -> Self {
        self.apply_with(|s| s.on_pressed = on_pressed.into())
    }

    /// Set whether the button is disabled.
    pub fn with_disabled(self, disabled: bool) -> Self {
        self.apply_with(|s| s.disabled = disabled)
    }

    /// Enable or disable auto-repeat when held down.
    pub fn with_repeat(self, enabled: bool) -> Self {
        self.apply_with(|s| s.repeat_enabled = enabled)
    }

    /// Set the initial delay before repeating starts (in milliseconds).
    pub fn with_repeat_delay(self, delay_ms: u64) -> Self {
        self.apply_with(|s| s.repeat_delay = Duration::from_millis(delay_ms))
    }

    /// Set the interval between repeats (in milliseconds).
    pub fn with_repeat_interval(self, interval_ms: u64) -> Self {
        self.apply_with(|s| s.repeat_interval = Duration::from_millis(interval_ms))
    }

    /// Set the theme style ID for this button.
    ///
    /// This allows the button to be styled differently by the theme.
    /// For example, a toolbar button might use "ToolbarButton".
    pub fn with_style_id(self, id: &'static str) -> Self {
        self.apply_with(|s| s.style_id = id)
    }

    /// Set whether to invert text color (for transparent backgrounds, set to false)
    pub fn with_invert_text(self, invert: bool) -> Self {
        self.apply_with(|s| s.invert_text = invert)
    }

    /// Set the tooltip text for this button.
    pub fn with_tooltip(self, tooltip: impl Into<String>) -> Self {
        self.apply_with(|s| s.tooltip = Some(tooltip.into()))
    }

    /// Set the status bar tip text for this button.
    ///
    /// The status tip is shown in the status bar when hovering over the button.
    /// This is different from tooltips, which appear as popups near the cursor.
    pub fn with_status_tip(self, status_tip: impl Into<String>) -> Self {
        self.apply_with(|s| s.status_tip = Some(status_tip.into()))
    }

    /// Set the layout style for this button.
    pub fn with_layout_style(self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.apply_with(|s| s.layout_style = layout_style.into())
    }

    fn update_focus_state(
        &mut self,
        layout: &LayoutNode,
        info: &mut AppInfo,
        old_focus_state: FocusState,
    ) {
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

            let new_focus_state = manager.get_focus_state(self.focus_id);
            if matches!(new_focus_state, FocusState::Gained)
                && !matches!(old_focus_state, FocusState::Focused)
            {
                self.focus_via_keyboard = manager.was_last_focus_via_keyboard();
            } else if matches!(new_focus_state, FocusState::Lost | FocusState::None) {
                self.focus_via_keyboard = false;
            }

            self.focus_state = new_focus_state;
        }
    }

    fn hit_test(&self, layout: &LayoutNode, cursor: Vector2<f64>) -> bool {
        let x = cursor.x;
        let y = cursor.y;
        // Add a small buffer zone to make tooltips less sensitive to cursor jitter
        const TOOLTIP_BUFFER: f64 = 2.0;
        let left = layout.layout.location.x as f64 - TOOLTIP_BUFFER;
        let top = layout.layout.location.y as f64 - TOOLTIP_BUFFER;
        let right = left + layout.layout.size.width as f64 + (TOOLTIP_BUFFER * 2.0);
        let bottom = top + layout.layout.size.height as f64 + (TOOLTIP_BUFFER * 2.0);

        x >= left && x <= right && y >= top && y <= bottom
    }

    fn handle_keyboard_input(&mut self, info: &AppInfo) -> Update {
        let mut update = Update::empty();

        if matches!(self.focus_state, FocusState::Focused | FocusState::Gained) {
            for (_, key_event) in &info.keys {
                match key_event.state {
                    ElementState::Pressed => match key_event.physical_key {
                        PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter) => {
                            update |= *self.on_pressed.get();
                            self.state = ButtonState::Pressed;
                        },
                        _ => {},
                    },
                    ElementState::Released => match key_event.physical_key {
                        PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter) => {
                            if self.state == ButtonState::Pressed {
                                self.state = ButtonState::Idle;
                            }
                        },
                        _ => {},
                    },
                }
            }
        }

        update
    }

    fn handle_mouse_input(
        &mut self,
        layout: &LayoutNode,
        info: &AppInfo,
    ) -> Update {
        let mut update = Update::empty();

        let cursor_hit = info
            .cursor_pos
            .map(|cursor| self.hit_test(layout, cursor))
            .unwrap_or(false);

        if !cursor_hit {
            self.state = ButtonState::Idle;
            return update;
        }

        if self.state != ButtonState::Pressed {
            self.state = ButtonState::Hovered;
        }

        for (_, btn, el) in &info.buttons {
            if *btn != MouseButton::Left {
                continue;
            }

            match el {
                ElementState::Pressed => {
                    if self.state != ButtonState::Pressed {
                        self.state = ButtonState::Pressed;
                        if self.repeat_enabled {
                            update |= *self.on_pressed.get();
                            self.press_start_time = Some(Instant::now());
                            self.last_repeat_time = None;
                        }
                    }
                },
                ElementState::Released => {
                    if self.state == ButtonState::Pressed {
                        self.state = ButtonState::Released;
                        if !self.repeat_enabled {
                            update |= *self.on_pressed.get();
                        }
                    }
                },
            }
        }

        update
    }

    fn handle_repeat(&mut self, update: &mut Update) {
        if !(self.repeat_enabled && self.state == ButtonState::Pressed) {
            self.press_start_time = None;
            self.last_repeat_time = None;
            return;
        }

        let now = Instant::now();
        if let Some(start_time) = self.press_start_time {
            if now.duration_since(start_time) >= self.repeat_delay {
                let should_repeat = self
                    .last_repeat_time
                    .map(|last| now.duration_since(last) >= self.repeat_interval)
                    .unwrap_or(true);

                if should_repeat {
                    *update |= *self.on_pressed.get();
                    self.last_repeat_time = Some(now);
                }
            }

            *update |= Update::DRAW;
        } else {
            self.press_start_time = Some(now);
        }
    }
}

impl WidgetChildExt for Button {
    fn set_child(&mut self, child: impl Widget + 'static) {
        self.child = Box::new(child);
    }
}

impl WidgetLayoutExt for Button {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

#[async_trait(?Send)]
impl Widget for Button {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Update focus state from focus manager
        if let Ok(mut manager) = info.focus_manager.lock() {
            self.focus_state = manager.get_focus_state(self.focus_id);
        }

        // Use centralized theme rendering (all themes support it via supertrait)
        let is_focused = matches!(self.focus_state, FocusState::Focused | FocusState::Gained)
            && self.focus_via_keyboard;
        render_button_with_theme(
            theme,
            &self.widget_id(),
            self.state,
            self.focus_state,
            is_focused,
            self.disabled,
            layout_node,
            graphics,
        );

        // Render child widget
        {
            theme.globals_mut().invert_text_color = self.invert_text;

            let mut child_scene = nptk_core::vg::Scene::new();
            let mut child_graphics = VelloGraphics::new(&mut child_scene);

            let child_layout = &layout_node.children[0];
            
            // Render child to scene - child layout coordinates are already relative to button
            self.child.render(
                &mut child_graphics,
                theme,
                child_layout,
                info,
                context,
            );

            // Apply clipping to prevent label overflow
            // Child layout coordinates are in screen space, so we clip to button bounds
            let button_rect = Rect::new(
                layout_node.layout.location.x as f64,
                layout_node.layout.location.y as f64,
                (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
            );

            graphics.push_layer(
                Mix::Clip,
                1.0,
                Affine::IDENTITY,
                &button_rect.to_path(0.1),
            );

            // Append without translation - child layout coordinates are already in screen space
            graphics.append(&child_scene, None);

            graphics.pop_layer();

            theme.globals_mut().invert_text_color = false;
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.child.layout_style()],
        }
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let old_state = self.state;
        let old_focus_state = self.focus_state;

        self.update_focus_state(layout, info, old_focus_state);

        if !self.disabled {
            update |= self.handle_keyboard_input(info);
            update |= self.handle_mouse_input(layout, info);

            // Handle tooltip requests
            if let Some(tooltip_text) = &self.tooltip {
                let cursor_hit = info
                    .cursor_pos
                    .map(|cursor| self.hit_test(layout, cursor))
                    .unwrap_or(false);

                // Only send requests when hover state actually changes
                if cursor_hit && !self.tooltip_hover_state {
                    // Entering hover state
                    self.tooltip_hover_state = true;
                    if let Some(cursor_pos) = info.cursor_pos {
                        context.request_tooltip_show(
                            tooltip_text.clone(),
                            self.widget_id(),
                            (cursor_pos.x, cursor_pos.y),
                        );
                    }
                } else if !cursor_hit && self.tooltip_hover_state {
                    self.tooltip_hover_state = false;
                    context.request_tooltip_hide();
                }
            }

            // Handle status bar tip updates
            if let Some(status_tip_text) = &self.status_tip {
                let cursor_hit = info
                    .cursor_pos
                    .map(|cursor| self.hit_test(layout, cursor))
                    .unwrap_or(false);

                // Only update status bar when hover state actually changes
                if cursor_hit && !self.status_tip_hover_state {
                    // Entering hover state
                    self.status_tip_hover_state = true;
                    context.set_status_bar_text(status_tip_text.clone());
                } else if !cursor_hit && self.status_tip_hover_state {
                    // Leaving hover state
                    self.status_tip_hover_state = false;
                    context.clear_status_bar_text();
                }
            }
        } else {
            self.state = ButtonState::Idle;
            self.press_start_time = None;
            self.last_repeat_time = None;
            self.tooltip_hover_state = false;
            context.request_tooltip_hide();
            self.status_tip_hover_state = false;
            context.clear_status_bar_text();
        }

        if old_state != self.state {
            update |= Update::DRAW;
        }

        if old_focus_state != self.focus_state {
            update |= Update::DRAW;
        }

        self.handle_repeat(&mut update);

        if !layout.children.is_empty() {
            update |= self.child.update(&layout.children[0], context, info).await;
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", self.style_id)
    }

    fn tooltip(&self) -> Option<String> {
        self.tooltip.clone()
    }

    fn set_tooltip(&mut self, tooltip: Option<String>) {
        self.tooltip = tooltip;
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
