use crate::theme_rendering::render_button_with_theme;
use std::time::{Duration, Instant};
use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusBounds, FocusId, FocusProperties, FocusState, FocusableWidget};
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Vec2};
use nptk_core::vgi::{vello_vg::VelloGraphics, Graphics};
use nptk_core::widget::{BoxedWidget, Widget, WidgetChildExt, WidgetLayoutExt};
use nptk_core::window::{ElementState, KeyCode, MouseButton, PhysicalKey};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

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
        }
    }

    /// Sets the function to be called when the button is pressed.
    pub fn with_on_pressed(mut self, on_pressed: impl Into<MaybeSignal<Update>>) -> Self {
        self.on_pressed = on_pressed.into();
        self
    }

    /// Set whether the button is disabled.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Enable or disable auto-repeat when held down.
    pub fn with_repeat(mut self, enabled: bool) -> Self {
        self.repeat_enabled = enabled;
        self
    }

    /// Set the initial delay before repeating starts (in milliseconds).
    pub fn with_repeat_delay(mut self, delay_ms: u64) -> Self {
        self.repeat_delay = Duration::from_millis(delay_ms);
        self
    }

    /// Set the interval between repeats (in milliseconds).
    pub fn with_repeat_interval(mut self, interval_ms: u64) -> Self {
        self.repeat_interval = Duration::from_millis(interval_ms);
        self
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
        if let Ok(manager) = info.focus_manager.lock() {
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
            theme.globals_mut().invert_text_color = true;

            let mut child_scene = nptk_core::vg::Scene::new();
            let mut child_graphics = VelloGraphics::new(&mut child_scene);

            self.child.render(
                &mut child_graphics,
                theme,
                &layout_node.children[0],
                info,
                context,
            );

            graphics.append(
                &child_scene,
                Some(Affine::translate(Vec2::new(
                    layout_node.layout.location.x as f64,
                    layout_node.layout.location.y as f64,
                ))),
            );

            theme.globals_mut().invert_text_color = false;
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
            if matches!(new_focus_state, FocusState::Gained)
                && !matches!(old_focus_state, FocusState::Focused)
            {
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
                            PhysicalKey::Code(KeyCode::Space)
                            | PhysicalKey::Code(KeyCode::Enter) => {
                                // Trigger button press via keyboard
                                update |= *self.on_pressed.get();
                                self.state = ButtonState::Pressed;
                            },
                            _ => {},
                        }
                    },
                    ElementState::Released => {
                        match key_event.physical_key {
                            PhysicalKey::Code(KeyCode::Space)
                            | PhysicalKey::Code(KeyCode::Enter) => {
                                // Reset button state after keyboard release
                                if self.state == ButtonState::Pressed {
                                    self.state = ButtonState::Idle;
                                }
                            },
                            _ => {},
                        }
                    },
                }
            }
        }

        // check for hovering
        if let Some(cursor) = info.cursor_pos {
            if cursor.x as f32 >= layout.layout.location.x
                && cursor.x as f32 <= layout.layout.location.x + layout.layout.size.width
                && cursor.y as f32 >= layout.layout.location.y
                && cursor.y as f32 <= layout.layout.location.y + layout.layout.size.height
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
                                if self.state != ButtonState::Pressed {
                                    self.state = ButtonState::Pressed;
                                    // If repeat is enabled, fire immediately on press too
                                    if self.repeat_enabled {
                                        update |= *self.on_pressed.get();
                                        self.press_start_time = Some(Instant::now());
                                        self.last_repeat_time = None;
                                    }
                                }
                            },

                            // actually fire the event if the button is released
                            ElementState::Released => {
                                self.state = ButtonState::Released;
                                // Only fire on release if repeat is NOT enabled
                                // (If repeat is enabled, we already fired on press and during hold)
                                if !self.repeat_enabled {
                                    update |= *self.on_pressed.get();
                                } else {
                                    // Reset repeat state
                                    self.press_start_time = None;
                                    self.last_repeat_time = None;
                                }
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

        // Handle repeat logic
        if self.repeat_enabled && self.state == ButtonState::Pressed {
            let now = Instant::now();
            
            if let Some(start_time) = self.press_start_time {
                // Check if we've passed the initial delay
                if now.duration_since(start_time) >= self.repeat_delay {
                    // Check if we've passed the interval since last repeat
                    let should_repeat = if let Some(last_repeat) = self.last_repeat_time {
                        now.duration_since(last_repeat) >= self.repeat_interval
                    } else {
                        // First repeat after delay
                        true
                    };

                    if should_repeat {
                        update |= *self.on_pressed.get();
                        self.last_repeat_time = Some(now);
                    }
                }
                
                // Keep requesting updates while pressed to ensure smooth repeating
                // We use Update::empty() here because we just need the loop to continue,
                // but usually we need to signal that we want another frame or update cycle.
                // In this framework, returning Update::DRAW or similar might be needed if we want to ensure
                // we get called back efficiently. However, update() is called on events.
                // To support timer-based updates without events, we might need a way to request a timer.
                // For now, we'll rely on the fact that if we change something, we get updated.
                // But if nothing changes (mouse held still), we might not get updates.
                // We can force a redraw which usually triggers a new update cycle in many game loops,
                // but here we should be careful. 
                // Ideally, we'd request a timer callback. Since we don't have that explicit API here yet,
                // we can return Update::DRAW to force a continuous loop while pressed, 
                // or rely on the fact that we are modifying state.
                // Let's try returning Update::DRAW to ensure we get called back.
                update |= Update::DRAW; 
            } else {
                // Should have been set when state became Pressed, but just in case
                self.press_start_time = Some(now);
            }
        } else if self.state != ButtonState::Pressed {
            // Reset repeat state when not pressed
            self.press_start_time = None;
            self.last_repeat_time = None;
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Button")
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
