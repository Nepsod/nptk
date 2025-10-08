use nptk_core::app::context::AppContext;
use nptk_core::app::focus::{FocusId, FocusState, FocusProperties, FocusBounds, FocusableWidget};
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode, Dimension};
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::vg::kurbo::{Affine, Circle, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, KeyCode, PhysicalKey, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;

/// Represents the state of a radio button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioButtonState {
    /// Radio button is in its default state
    Idle,
    /// Mouse cursor is hovering over the radio button
    Hovered,
    /// Mouse button is pressed down on the radio button
    Pressed,
}

/// A radio button widget for mutually exclusive selections.
/// 
/// Radio buttons are typically used in groups where only one option can be selected at a time.
/// Use the same `group_id` for buttons that should be mutually exclusive.
pub struct RadioButton {
    /// Whether this radio button is selected
    selected: StateSignal<bool>,
    /// Text label for the radio button
    label: MaybeSignal<String>,
    /// Group identifier for mutual exclusion
    group_id: String,
    /// Callback when the radio button is selected
    on_selected: Option<Box<dyn Fn() + Send + Sync>>,
    /// Current visual state
    state: RadioButtonState,
    /// Layout styling
    layout_style: MaybeSignal<LayoutStyle>,
    /// Focus management
    focus_id: FocusId,
    focus_state: FocusState,
    focus_via_keyboard: bool,
    /// Whether the radio button is disabled
    disabled: bool,
}

impl RadioButton {
    /// Create a new radio button.
    pub fn new(label: impl Into<MaybeSignal<String>>, group_id: String) -> Self {
        Self {
            selected: StateSignal::new(false),
            label: label.into(),
            group_id,
            on_selected: None,
            state: RadioButtonState::Idle,
            layout_style: MaybeSignal::value(LayoutStyle {
            size: Vector2::new(Dimension::length(24.0), Dimension::length(24.0)),
            ..Default::default()
        }),
            focus_id: FocusId::new(),
            focus_state: FocusState::None,
            focus_via_keyboard: false,
            disabled: false,
        }
    }

    /// Set the initial selected state.
    pub fn with_selected(self, selected: bool) -> Self {
        self.selected.set(selected);
        self
    }

    /// Set a callback for when the radio button is selected.
    pub fn with_on_selected<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_selected = Some(Box::new(callback));
        self
    }

    /// Set whether the radio button is disabled.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Get the current selected state.
    pub fn selected(&self) -> &StateSignal<bool> {
        &self.selected
    }

    /// Get the group ID.
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    /// Set the selected state and trigger callback if selected.
    pub fn set_selected(&mut self, selected: bool) {
        let was_selected = *self.selected.get();
        self.selected.set(selected);
        
        if selected && !was_selected {
            if let Some(callback) = &self.on_selected {
                callback();
            }
        }
    }
}

impl WidgetLayoutExt for RadioButton {
    fn with_layout_style(mut self, style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = style.into();
        self
    }

    fn set_layout_style(&mut self, style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = style.into();
    }
}

impl Widget for RadioButton {
    fn render(&mut self, scene: &mut Scene, theme: &mut dyn Theme, layout_node: &LayoutNode, info: &mut AppInfo, _context: AppContext) {
        
        // Update focus state
        if let Ok(manager) = info.focus_manager.lock() {
            self.focus_state = manager.get_focus_state(self.focus_id);
        }

        let is_focused = matches!(self.focus_state, FocusState::Focused | FocusState::Gained);
        let is_selected = *self.selected.get();
        

        // Get colors from theme
        let radio_size = 16.0;
        let radio_center_x = layout_node.layout.location.x + radio_size / 2.0 + 4.0;
        let radio_center_y = layout_node.layout.location.y + layout_node.layout.size.height / 2.0;

        // Radio button circle colors
        let bg_color = if self.disabled {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundDisabled)
                .unwrap_or_else(|| Color::from_rgb8(240, 240, 240))
        } else if is_selected {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                .unwrap_or_else(|| Color::WHITE)
        } else {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackground)
                .unwrap_or_else(|| Color::WHITE)
        };

        let border_color = if self.disabled {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBorderDisabled)
                .unwrap_or_else(|| Color::from_rgb8(200, 200, 200))
        } else if is_focused && self.focus_via_keyboard {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBorderFocused)
                .unwrap_or_else(|| Color::from_rgb8(0, 120, 255))
        } else if matches!(self.state, RadioButtonState::Hovered) {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBorderHovered)
                .unwrap_or_else(|| Color::from_rgb8(100, 100, 100))
        } else {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBorder)
                .unwrap_or_else(|| Color::from_rgb8(150, 150, 150))
        };

        let dot_color = if self.disabled {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorDotDisabled)
                .unwrap_or_else(|| Color::from_rgb8(180, 180, 180))
        } else {
            theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorDot)
                .unwrap_or_else(|| Color::from_rgb8(0, 120, 255))
        };

        // Draw radio button circle background
        let radio_circle = Circle::new((radio_center_x as f64, radio_center_y as f64), radio_size as f64 / 2.0);
        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &Brush::Solid(bg_color),
            None,
            &radio_circle,
        );

        // Draw radio button border
        let border_width = if is_focused && self.focus_via_keyboard { 2.0 } else { 1.0 };
        scene.stroke(
            &Stroke::new(border_width),
            Affine::default(),
            &Brush::Solid(border_color),
            None,
            &radio_circle,
        );

        // Draw selected dot
        if is_selected {
            let dot_size = radio_size * 0.6; // Make dot bigger and more visible
            let dot_circle = Circle::new((radio_center_x as f64, radio_center_y as f64), dot_size as f64 / 2.0);
            scene.fill(
                Fill::NonZero,
                Affine::default(),
                &Brush::Solid(dot_color),
                None,
                &dot_circle,
            );
        }

        // Draw label text
        let label_text = self.label.get();
        if !label_text.is_empty() {
            let _text_color = if self.disabled {
                theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorTextDisabled)
                    .unwrap_or_else(|| Color::from_rgb8(150, 150, 150))
            } else {
                theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                    .unwrap_or_else(|| Color::from_rgb8(0, 0, 0))
            };

            // TODO: Implement text rendering similar to other widgets
            // For now, we'll skip text rendering to avoid font complexity
            // This would need the same font rendering approach as TextInput/Button
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Register with focus manager
        if let Ok(mut manager) = info.focus_manager.lock() {
            let focusable_widget = FocusableWidget {
                id: self.focus_id,
                properties: FocusProperties {
                    tab_focusable: !self.disabled,
                    click_focusable: !self.disabled,
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

            // Update focus state
            let new_focus_state = manager.get_focus_state(self.focus_id);
            if new_focus_state != self.focus_state {
                self.focus_state = new_focus_state;
                
                if matches!(self.focus_state, FocusState::Gained) {
                    self.focus_via_keyboard = manager.was_last_focus_via_keyboard();
                }
                
                update |= Update::DRAW;
            }
        }

        let is_focused = matches!(self.focus_state, FocusState::Focused | FocusState::Gained);
        
        // Handle keyboard input when focused
        if is_focused && !self.disabled {
            for (_device_id, key_event) in &info.keys {
                if key_event.state == ElementState::Pressed {
                    match key_event.physical_key {
                        PhysicalKey::Code(KeyCode::Space) | PhysicalKey::Code(KeyCode::Enter) => {
                            if !*self.selected.get() {
                                self.set_selected(true);
                                // TODO: Deselect other radio buttons in the same group
                                update |= Update::DRAW;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Handle mouse input
        if let Some(cursor_pos) = info.cursor_pos {
            let in_bounds = cursor_pos.x >= layout.layout.location.x as f64
                && cursor_pos.x <= (layout.layout.location.x + layout.layout.size.width) as f64
                && cursor_pos.y >= layout.layout.location.y as f64
                && cursor_pos.y <= (layout.layout.location.y + layout.layout.size.height) as f64;

            if !self.disabled {
                if in_bounds {
                    match self.state {
                        RadioButtonState::Idle => {
                            self.state = RadioButtonState::Hovered;
                            update |= Update::DRAW;
                        }
                        RadioButtonState::Pressed => {
                            // Keep pressed state while mouse is down
                        }
                        RadioButtonState::Hovered => {
                            // Already hovered
                        }
                    }
                } else {
                    if matches!(self.state, RadioButtonState::Hovered) {
                        self.state = RadioButtonState::Idle;
                        update |= Update::DRAW;
                    }
                }
            }
        } else {
            // Mouse left the window
            if matches!(self.state, RadioButtonState::Hovered) {
                self.state = RadioButtonState::Idle;
                update |= Update::DRAW;
            }
        }

        // Handle mouse button events globally (not just when in bounds)
        if !self.disabled {
            for (_device_id, button, state) in &info.buttons {
                if *button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            if let Some(cursor_pos) = info.cursor_pos {
                                let in_bounds = cursor_pos.x >= layout.layout.location.x as f64
                                    && cursor_pos.x <= (layout.layout.location.x + layout.layout.size.width) as f64
                                    && cursor_pos.y >= layout.layout.location.y as f64
                                    && cursor_pos.y <= (layout.layout.location.y + layout.layout.size.height) as f64;
                                
                                if in_bounds {
                                    context.set_focus(Some(self.focus_id));
                                    self.state = RadioButtonState::Pressed;
                                    update |= Update::DRAW;
                                }
                            }
                        }
                        ElementState::Released => {
                            if matches!(self.state, RadioButtonState::Pressed) {
                                if let Some(cursor_pos) = info.cursor_pos {
                                    let in_bounds = cursor_pos.x >= layout.layout.location.x as f64
                                        && cursor_pos.x <= (layout.layout.location.x + layout.layout.size.width) as f64
                                        && cursor_pos.y >= layout.layout.location.y as f64
                                        && cursor_pos.y <= (layout.layout.location.y + layout.layout.size.height) as f64;
                                    
                                    if in_bounds && !*self.selected.get() {
                                        self.set_selected(true);
                                        // TODO: Deselect other radio buttons in the same group
                                        update |= Update::DRAW;
                                    }
                                }
                                
                                // Reset state regardless of bounds
                                if let Some(cursor_pos) = info.cursor_pos {
                                    let in_bounds = cursor_pos.x >= layout.layout.location.x as f64
                                        && cursor_pos.x <= (layout.layout.location.x + layout.layout.size.width) as f64
                                        && cursor_pos.y >= layout.layout.location.y as f64
                                        && cursor_pos.y <= (layout.layout.location.y + layout.layout.size.height) as f64;
                                    self.state = if in_bounds { RadioButtonState::Hovered } else { RadioButtonState::Idle };
                                } else {
                                    self.state = RadioButtonState::Idle;
                                }
                                update |= Update::DRAW;
                            }
                        }
                    }
                }
            }
        }

        update
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "RadioButton")
    }
}

impl Default for RadioButton {
    fn default() -> Self {
        Self::new("".to_string(), "default".to_string())
    }
}
