use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutContext, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{
    Affine, Line, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::helpers::ThemeHelper;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;

/// The state of a checkbox widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxState {
    /// Unchecked state
    Unchecked,
    /// Checked state  
    Checked,
    /// Indeterminate state (partially selected, like in Windows file trees)
    Indeterminate,
}

impl CheckboxState {
    /// Cycle to the next state in the sequence: Unchecked -> Checked -> Indeterminate -> Unchecked
    pub fn cycle_next(self) -> Self {
        match self {
            CheckboxState::Unchecked => CheckboxState::Checked,
            CheckboxState::Checked => CheckboxState::Indeterminate,
            CheckboxState::Indeterminate => CheckboxState::Unchecked,
        }
    }

    /// Cycle to the next state, skipping indeterminate if not allowed.
    /// For simple checkboxes: Unchecked -> Checked -> Unchecked
    /// For three-state checkboxes: Unchecked -> Checked -> Indeterminate -> Unchecked
    pub fn cycle_next_with_indeterminate(self, allow_indeterminate: bool) -> Self {
        if allow_indeterminate {
            self.cycle_next()
        } else {
            match self {
                CheckboxState::Unchecked => CheckboxState::Checked,
                CheckboxState::Checked => CheckboxState::Unchecked,
                CheckboxState::Indeterminate => CheckboxState::Unchecked, // Force to unchecked if somehow indeterminate
            }
        }
    }

    /// Convert to boolean for backward compatibility (true = checked, false = unchecked/indeterminate)
    pub fn to_bool(self) -> bool {
        matches!(self, CheckboxState::Checked)
    }

    /// Create from boolean for backward compatibility
    pub fn from_bool(value: bool) -> Self {
        if value {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        }
    }
}

/// A checkbox widget with three states: unchecked, checked, and indeterminate.
/// Changes state when clicked, cycling through: Unchecked -> Checked -> Indeterminate -> Unchecked
///
/// ### Theming
/// Styling the checkbox requires the following properties:
/// - `color_unchecked` - The color of the checkbox when unchecked
/// - `color_checked` - The color of the checkbox when checked  
/// - `color_indeterminate` - The color of the checkbox when indeterminate
///
/// ### State Locking
/// Each state can be individually locked to prevent cycling from that state.
/// When a state is locked, clicking the checkbox will not change its state.
pub struct Checkbox {
    layout_style: MaybeSignal<LayoutStyle>,
    value: MaybeSignal<CheckboxState>,
    on_change: MaybeSignal<Update>,
    locked_states: MaybeSignal<Vec<CheckboxState>>,
    allow_indeterminate: bool,
}

impl Checkbox {
    /// Create a new checkbox with the given state.
    ///
    /// The value should be a signal, so it's mutable.
    /// By default, indeterminate state is disabled for simple checkboxes.
    pub fn new(value: impl Into<MaybeSignal<CheckboxState>>) -> Self {
        Self {
            layout_style: LayoutStyle {
                size: Vector2::<Dimension>::new(Dimension::length(20.0), Dimension::length(20.0)),
                margin: layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(0.5),
                    right: LengthPercentageAuto::length(0.5),
                    top: LengthPercentageAuto::length(0.5),
                    bottom: LengthPercentageAuto::length(0.5),
                },
                ..Default::default()
            }
            .into(),
            value: value.into(),
            on_change: Update::empty().into(),
            locked_states: Vec::new().into(),
            allow_indeterminate: false,
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Create a new checkbox from a boolean value (for backward compatibility).
    ///
    /// The value should be a signal, so it's mutable.
    pub fn new_bool(value: impl Into<MaybeSignal<bool>>) -> Self {
        let bool_signal = value.into();
        Self::new(
            bool_signal.map(|b| nptk_core::reference::Ref::Owned(CheckboxState::from_bool(*b))),
        )
    }

    /// Enable the indeterminate state for this checkbox.
    ///
    /// This allows the checkbox to cycle through all three states:
    /// Unchecked -> Checked -> Indeterminate -> Unchecked
    ///
    /// Use this for master checkboxes that control multiple sub-items.
    pub fn with_indeterminate_state(self) -> Self {
        self.apply_with(|s| s.allow_indeterminate = true)
    }

    /// Sets the value of the checkbox and returns itself.
    pub fn with_value(self, value: impl Into<MaybeSignal<CheckboxState>>) -> Self {
        self.apply_with(|s| s.value = value.into())
    }

    /// Sets the update value to apply on changes.
    pub fn with_on_change(self, on_change: impl Into<MaybeSignal<Update>>) -> Self {
        self.apply_with(|s| s.on_change = on_change.into())
    }

    /// Lock specific states to prevent cycling from them.
    ///
    /// When a state is locked, clicking the checkbox will not change its state.
    ///
    /// # Arguments
    /// * `states` - A vector of states to lock
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_widgets::checkbox::{Checkbox, CheckboxState};
    ///
    /// let mut checkbox = Checkbox::new(CheckboxState::Unchecked);
    /// // Lock only the checked state
    /// checkbox = checkbox.with_locked_states(vec![CheckboxState::Checked]);
    ///
    /// // Lock multiple states
    /// checkbox = checkbox.with_locked_states(vec![
    ///     CheckboxState::Checked,
    ///     CheckboxState::Indeterminate,
    /// ]);
    ///
    /// // Lock all states (checkbox becomes completely unclickable)
    /// checkbox = checkbox.with_locked_states(vec![
    ///     CheckboxState::Unchecked,
    ///     CheckboxState::Checked,
    ///     CheckboxState::Indeterminate,
    /// ]);
    /// ```
    pub fn with_locked_states(
        self,
        states: impl Into<MaybeSignal<Vec<CheckboxState>>>,
    ) -> Self {
        self.apply_with(|s| s.locked_states = states.into())
    }

    /// Lock a single state to prevent cycling from it.
    ///
    /// This is a convenience method for locking just one state.
    ///
    /// # Arguments
    /// * `state` - The state to lock
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_widgets::checkbox::{Checkbox, CheckboxState};
    ///
    /// let mut checkbox = Checkbox::new(CheckboxState::Unchecked);
    /// // Lock the checked state
    /// checkbox = checkbox.with_locked_state(CheckboxState::Checked);
    /// ```
    pub fn with_locked_state(self, state: CheckboxState) -> Self {
        self.apply_with(|s| s.locked_states = vec![state].into())
    }

    /// Check if the current state is locked.
    ///
    /// Returns `true` if the current state is in the locked states list.
    pub fn is_current_state_locked(&self) -> bool {
        let current_state = *self.value.get();
        let locked_states = self.locked_states.get();
        locked_states.contains(&current_state)
    }

    /// Check if a specific state is locked.
    ///
    /// Returns `true` if the given state is in the locked states list.
    pub fn is_state_locked(&self, state: CheckboxState) -> bool {
        let locked_states = self.locked_states.get();
        locked_states.contains(&state)
    }

    /// Add a state to the locked states list.
    ///
    /// This method allows dynamic addition of locked states at runtime.
    /// Note: This only works if `locked_states` is a signal.
    pub fn lock_state(&mut self, state: CheckboxState) {
        if let Some(sig) = self.locked_states.as_signal() {
            let mut locked_states = sig.get().clone();
            if !locked_states.contains(&state) {
                locked_states.push(state);
                sig.set(locked_states);
            }
        }
    }

    /// Remove a state from the locked states list.
    ///
    /// This method allows dynamic removal of locked states at runtime.
    /// Note: This only works if `locked_states` is a signal.
    pub fn unlock_state(&mut self, state: CheckboxState) {
        if let Some(sig) = self.locked_states.as_signal() {
            let mut locked_states = sig.get().clone();
            locked_states.retain(|&s| s != state);
            sig.set(locked_states);
        }
    }

    /// Clear all locked states.
    ///
    /// This method removes all states from the locked states list.
    /// Note: This only works if `locked_states` is a signal.
    pub fn unlock_all_states(&mut self) {
        if let Some(sig) = self.locked_states.as_signal() {
            sig.set(Vec::new());
        }
    }

    fn checkbox_rect(layout_node: &LayoutNode) -> Rect {
        Rect::new(
            layout_node.layout.location.x as f64,
            layout_node.layout.location.y as f64,
            (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
            (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
        )
    }

    fn hit_test(layout_node: &LayoutNode, cursor: Vector2<f64>) -> bool {
        let rect = Self::checkbox_rect(layout_node);
        cursor.x >= rect.x0 && cursor.x <= rect.x1 && cursor.y >= rect.y0 && cursor.y <= rect.y1
    }

    fn current_state(&self) -> CheckboxState {
        *self.value.get()
    }

    fn next_state(&self, current: CheckboxState) -> CheckboxState {
        current.cycle_next_with_indeterminate(self.allow_indeterminate)
    }

    fn try_toggle(&mut self) -> bool {
        let current_state = self.current_state();
        if self.is_state_locked(current_state) {
            return false;
        }

        if let Some(sig) = self.value.as_signal() {
            let new_state = self.next_state(current_state);
            sig.set(new_state);
            true
        } else {
            false
        }
    }
}

impl WidgetLayoutExt for Checkbox {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

#[async_trait(?Send)]
impl Widget for Checkbox {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let state = self.current_state();

        // Check if current state is locked for graying out
        let is_locked = self.is_state_locked(state);

        // Get colors based on state using theme helper
        let theme_checkbox_state = match state {
            CheckboxState::Unchecked => nptk_theme::helpers::CheckboxState::Unchecked,
            CheckboxState::Checked => nptk_theme::helpers::CheckboxState::Checked,
            CheckboxState::Indeterminate => nptk_theme::helpers::CheckboxState::Indeterminate,
        };
        let base_border_color = ThemeHelper::get_checkbox_color_three_state(
            &*theme,
            self.widget_id(),
            theme_checkbox_state,
        );

        // Get symbol color from theme
        let base_symbol_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::CheckboxSymbol,
            )
            .unwrap_or(Color::from_rgb8(255, 255, 255));

        // Gray out colors if locked (like Windows disabled state)
        let border_color = if is_locked {
            Color::from_rgb8(150, 150, 150) // Light gray for locked/disabled appearance
        } else {
            base_border_color
        };

        let symbol_color = if is_locked {
            Color::from_rgb8(220, 220, 220) // Light gray for locked symbols
        } else {
            base_symbol_color
        };

        let fill_color = match state {
            CheckboxState::Unchecked => None,
            CheckboxState::Checked => Some(border_color),
            CheckboxState::Indeterminate => Some(border_color),
        };

        let checkbox_rect = Self::checkbox_rect(layout_node);

        let rounded_rect =
            RoundedRect::from_rect(checkbox_rect, RoundedRectRadii::from_single_radius(3.0));

        // Draw border with normal style (colors already grayed out if locked)
        let border_width = 2.0;

        graphics.stroke(
            &Stroke::new(border_width),
            Affine::default(),
            &Brush::Solid(border_color),
            None,
            &rounded_rect.to_path(0.1),
        );

        // Draw fill and symbols based on state
        match state {
            CheckboxState::Checked => {
                // Draw filled background
                let inner_rect = Rect::new(
                    checkbox_rect.x0 + 2.0,
                    checkbox_rect.y0 + 2.0,
                    checkbox_rect.x1 - 2.0,
                    checkbox_rect.y1 - 2.0,
                );
                let inner_rounded =
                    RoundedRect::from_rect(inner_rect, RoundedRectRadii::from_single_radius(2.0));
                graphics.fill(
                    Fill::NonZero,
                    Affine::default(),
                    &Brush::Solid(fill_color.unwrap()),
                    None,
                    &inner_rounded.to_path(0.1),
                );

                // Draw checkmark
                let center_x = (checkbox_rect.x0 + checkbox_rect.x1) / 2.0;
                let center_y = (checkbox_rect.y0 + checkbox_rect.y1) / 2.0;
                let size = checkbox_rect.width().min(checkbox_rect.height()) * 0.45;

                // Simple checkmark: two lines forming a V
                let line1 = Line::new(
                    Point::new(center_x - size * 0.5, center_y),
                    Point::new(center_x - size * 0.1, center_y + size * 0.4),
                );

                let line2 = Line::new(
                    Point::new(center_x - size * 0.1, center_y + size * 0.4),
                    Point::new(center_x + size * 0.6, center_y - size * 0.4),
                );

                graphics.stroke(
                    &Stroke::new(2.5),
                    Affine::default(),
                    &Brush::Solid(symbol_color),
                    None,
                    &line1.to_path(0.1),
                );
                graphics.stroke(
                    &Stroke::new(2.5),
                    Affine::default(),
                    &Brush::Solid(symbol_color),
                    None,
                    &line2.to_path(0.1),
                );
            },
            CheckboxState::Indeterminate => {
                // Draw filled background
                let inner_rect = Rect::new(
                    checkbox_rect.x0 + 2.0,
                    checkbox_rect.y0 + 2.0,
                    checkbox_rect.x1 - 2.0,
                    checkbox_rect.y1 - 2.0,
                );
                let inner_rounded =
                    RoundedRect::from_rect(inner_rect, RoundedRectRadii::from_single_radius(2.0));
                graphics.fill(
                    Fill::NonZero,
                    Affine::default(),
                    &Brush::Solid(fill_color.unwrap()),
                    None,
                    &inner_rounded.to_path(0.1),
                );

                // Draw horizontal line (minus sign) - only for indeterminate state
                let center_x = (checkbox_rect.x0 + checkbox_rect.x1) / 2.0;
                let center_y = (checkbox_rect.y0 + checkbox_rect.y1) / 2.0;
                let line_width = checkbox_rect.width() * 0.5;

                let line = Line::new(
                    Point::new(center_x - line_width / 2.0, center_y),
                    Point::new(center_x + line_width / 2.0, center_y),
                );

                graphics.stroke(
                    &Stroke::new(2.5),
                    Affine::default(),
                    &Brush::Solid(symbol_color),
                    None,
                    &line.to_path(0.1),
                );
            },
            CheckboxState::Unchecked => {
                // No fill, no symbols for unchecked state - just the border
            },
        }
    }

    fn layout_style(&self, _context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
            measure_func: None,
        }
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        let on_change = *self.on_change.get();

        let cursor_hit = info
            .cursor_pos
            .map(|cursor| Self::hit_test(layout, cursor))
            .unwrap_or(false);

        if cursor_hit {
            for (_, btn, el) in &info.buttons {
                if *btn == MouseButton::Left && *el == ElementState::Released {
                    if self.try_toggle() {
                        update |= on_change;
                        update |= Update::DRAW;
                    }
                }
            }
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Checkbox")
    }
}
