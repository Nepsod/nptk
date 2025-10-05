use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke, Line, Point};
use nptk_core::vg::peniko::{Brush, Fill, Color};
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nptk_theme::helpers::ThemeHelper;
use nalgebra::Vector2;

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
pub struct Checkbox {
    layout_style: MaybeSignal<LayoutStyle>,
    value: MaybeSignal<CheckboxState>,
    on_change: MaybeSignal<Update>,
}

impl Checkbox {
    /// Create a new checkbox with the given state.
    ///
    /// The value should be a signal, so it's mutable.
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
        }
    }

    /// Create a new checkbox from a boolean value (for backward compatibility).
    ///
    /// The value should be a signal, so it's mutable.
    pub fn new_bool(value: impl Into<MaybeSignal<bool>>) -> Self {
        let bool_signal = value.into();
        Self::new(bool_signal.map(|b| nptk_core::reference::Ref::Owned(CheckboxState::from_bool(*b))))
    }

    /// Sets the value of the checkbox and returns itself.
    pub fn with_value(mut self, value: impl Into<MaybeSignal<CheckboxState>>) -> Self {
        self.value = value.into();
        self
    }

    /// Sets the update value to apply on changes.
    pub fn with_on_change(mut self, on_change: impl Into<MaybeSignal<Update>>) -> Self {
        self.on_change = on_change.into();
        self
    }
}

impl WidgetLayoutExt for Checkbox {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for Checkbox {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let state = *self.value.get();
        
        // Get colors based on state using theme helper
        let theme_checkbox_state = match state {
            CheckboxState::Unchecked => nptk_theme::helpers::CheckboxState::Unchecked,
            CheckboxState::Checked => nptk_theme::helpers::CheckboxState::Checked,
            CheckboxState::Indeterminate => nptk_theme::helpers::CheckboxState::Indeterminate,
        };
        let border_color = ThemeHelper::get_checkbox_color_three_state(&*theme, self.widget_id(), theme_checkbox_state);
        let fill_color = match state {
            CheckboxState::Unchecked => None,
            CheckboxState::Checked => Some(border_color),
            CheckboxState::Indeterminate => Some(border_color),
        };

        let checkbox_rect = Rect::new(
            layout_node.layout.location.x as f64,
            layout_node.layout.location.y as f64,
            (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
            (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
        );

        let rounded_rect = RoundedRect::from_rect(checkbox_rect, RoundedRectRadii::from_single_radius(3.0));

        // Draw border
        scene.stroke(
            &Stroke::new(2.0),
            Affine::default(),
            &Brush::Solid(border_color),
            None,
            &rounded_rect,
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
                let inner_rounded = RoundedRect::from_rect(inner_rect, RoundedRectRadii::from_single_radius(2.0));
                scene.fill(Fill::NonZero, Affine::default(), &Brush::Solid(fill_color.unwrap()), None, &inner_rounded);
                
                // Draw checkmark
                let center_x = (checkbox_rect.x0 + checkbox_rect.x1) / 2.0;
                let center_y = (checkbox_rect.y0 + checkbox_rect.y1) / 2.0;
                let size = checkbox_rect.width().min(checkbox_rect.height()) * 0.25;
                
                // Simple checkmark: two lines forming a V
                let line1 = Line::new(
                    Point::new(center_x - size * 0.5, center_y),
                    Point::new(center_x - size * 0.1, center_y + size * 0.4),
                );
                
                let line2 = Line::new(
                    Point::new(center_x - size * 0.1, center_y + size * 0.4),
                    Point::new(center_x + size * 0.6, center_y - size * 0.4),
                );
                
                scene.stroke(&Stroke::new(2.0), Affine::default(), &Brush::Solid(Color::WHITE), None, &line1);
                scene.stroke(&Stroke::new(2.0), Affine::default(), &Brush::Solid(Color::WHITE), None, &line2);
            }
            CheckboxState::Indeterminate => {
                // Draw filled background
                let inner_rect = Rect::new(
                    checkbox_rect.x0 + 2.0,
                    checkbox_rect.y0 + 2.0,
                    checkbox_rect.x1 - 2.0,
                    checkbox_rect.y1 - 2.0,
                );
                let inner_rounded = RoundedRect::from_rect(inner_rect, RoundedRectRadii::from_single_radius(2.0));
                scene.fill(Fill::NonZero, Affine::default(), &Brush::Solid(fill_color.unwrap()), None, &inner_rounded);
                
                // Draw horizontal line (minus sign) - only for indeterminate state
                let center_x = (checkbox_rect.x0 + checkbox_rect.x1) / 2.0;
                let center_y = (checkbox_rect.y0 + checkbox_rect.y1) / 2.0;
                let line_width = checkbox_rect.width() * 0.5;
                
                let line = Line::new(
                    Point::new(center_x - line_width / 2.0, center_y),
                    Point::new(center_x + line_width / 2.0, center_y),
                );
                
                scene.stroke(&Stroke::new(2.5), Affine::default(), &Brush::Solid(Color::WHITE), None, &line);
            }
            CheckboxState::Unchecked => {
                // No fill, no symbols for unchecked state - just the border
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, layout: &LayoutNode, _: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        if let Some(cursor) = &info.cursor_pos {
            if cursor.x as f32 >= layout.layout.location.x
                && cursor.x as f32 <= layout.layout.location.x + layout.layout.size.width
                && cursor.y as f32 >= layout.layout.location.y
                && cursor.y as f32 <= layout.layout.location.y + layout.layout.size.height
            {
                for (_, btn, el) in &info.buttons {
                    if btn == &MouseButton::Left && *el == ElementState::Released {
                        update |= *self.on_change.get();
                        update |= Update::DRAW;

                        if let Some(sig) = self.value.as_signal() {
                            let current_state = *sig.get();
                            let new_state = current_state.cycle_next();
                            sig.set(new_state);
                        }
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
