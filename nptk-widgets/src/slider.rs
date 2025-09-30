use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Circle, Point, Rect, RoundedRect, RoundedRectRadii};
use nptk_core::vg::peniko::{Brush, Fill};
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::window::MouseButton;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;

/// A slider widget to control a floating point value between `0.0` and `1.0`.
///
/// ### Theming
/// You can style the slider using following properties:
/// - `color` - The color of the slider bar.
/// - `color_ball` - The color of the slider ball.
pub struct Slider {
    layout_style: MaybeSignal<LayoutStyle>,
    value: MaybeSignal<f32>,
    on_change: MaybeSignal<Update>,
    dragging: bool,
}

impl Slider {
    /// Create a new Slider widget from a value (should be a signal) and an `on_change` callback.
    pub fn new(value: impl Into<MaybeSignal<f32>>) -> Self {
        Self {
            layout_style: LayoutStyle {
                size: Vector2::<Dimension>::new(Dimension::length(100.0), Dimension::length(10.0)),
                margin: layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(10.0),
                    right: LengthPercentageAuto::length(0.0),
                    top: LengthPercentageAuto::length(10.0),
                    bottom: LengthPercentageAuto::length(10.0),
                },
                ..Default::default()
            }
            .into(),
            value: value.into(),
            on_change: MaybeSignal::value(Update::empty()),
            dragging: false,
        }
    }

    /// Sets the layout style of the slider and returns itself.
    pub fn with_value(mut self, value: impl Into<MaybeSignal<f32>>) -> Self {
        self.value = value.into();
        self
    }

    /// Sets the function to be called when the slider is clicked/changed.
    pub fn with_on_change(mut self, on_change: impl Into<MaybeSignal<Update>>) -> Self {
        self.on_change = on_change.into();
        self
    }
}

impl WidgetLayoutExt for Slider {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for Slider {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let value = *self.value.get();

        let brush = if let Some(style) = theme.of(self.widget_id()) {
            Brush::Solid(style.get_color("color").unwrap())
        } else {
            Brush::Solid(theme.defaults().interactive().inactive())
        };

        let ball_brush = if let Some(style) = theme.of(self.widget_id()) {
            Brush::Solid(style.get_color("color_ball").unwrap())
        } else {
            Brush::Solid(theme.defaults().interactive().active())
        };

        let circle_radius = layout_node.layout.size.height as f64 / 1.15;

        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &brush,
            None,
            &RoundedRect::from_rect(
                Rect::new(
                    layout_node.layout.location.x as f64,
                    layout_node.layout.location.y as f64,
                    (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                    (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
                ),
                RoundedRectRadii::from_single_radius(20.0),
            ),
        );

        scene.fill(
            Fill::NonZero,
            Affine::default(),
            &ball_brush,
            None,
            &Circle::new(
                Point::new(
                    (layout_node.layout.location.x + layout_node.layout.size.width * value) as f64,
                    (layout_node.layout.location.y + layout_node.layout.size.height / 2.0) as f64,
                ),
                circle_radius,
            ),
        );
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, layout: &LayoutNode, _: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        if let Some(cursor) = info.cursor_pos {
            if cursor.x as f32 >= layout.layout.location.x
                && cursor.x as f32 <= layout.layout.location.x + layout.layout.size.width
                && cursor.y as f32 >= layout.layout.location.y
                && cursor.y as f32 <= layout.layout.location.y + layout.layout.size.height
            {
                for (_, btn, el_state) in &info.buttons {
                    if btn == &MouseButton::Left && el_state.is_pressed() {
                        self.dragging = el_state.is_pressed();
                    }
                }

                if self.dragging {
                    let new_value =
                        (cursor.x as f32 - layout.layout.location.x) / layout.layout.size.width;

                    if let Some(sig) = self.value.as_signal() {
                        sig.set(new_value);
                    }

                    update.insert(*self.on_change.get());
                    update.insert(Update::DRAW);
                }
            }
        } else {
            self.dragging = false;
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Slider")
    }
}
