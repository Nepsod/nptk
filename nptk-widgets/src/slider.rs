use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Shape};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
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
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        _: AppContext,
    ) {
        let value = *self.value.get();

        // Background track color (unfilled portion)
        let track_brush = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::Color)
            .map(|color| Brush::Solid(color))
            .unwrap_or_else(|| Brush::Solid(Color::from_rgb8(200, 200, 200)));

        // Filled track color (up to thumb position) - using primary-dark
        // For Sweet theme, primary-dark is (157, 51, 213), but we use primary as fallback
        // Ideally we'd have a ThemeProperty for filled track, but for now use primary-dark directly
        let filled_track_color = Color::from_rgb8(157, 51, 213); // primary-dark

        let thumb_brush = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBall)
            .map(|color| Brush::Solid(color))
            .unwrap_or_else(|| Brush::Solid(Color::from_rgb8(100, 150, 255)));

        let track_height = 3.0; // Thin track
        let track_center_y = (layout_node.layout.location.y + layout_node.layout.size.height / 2.0) as f64;
        let track_top = track_center_y - track_height / 2.0;
        let track_bottom = track_center_y + track_height / 2.0;

        // Draw background track (full width)
        graphics.fill(
            Fill::NonZero,
            Affine::default(),
            &track_brush,
            None,
            &RoundedRect::from_rect(
                Rect::new(
                    layout_node.layout.location.x as f64,
                    track_top,
                    (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                    track_bottom,
                ),
                RoundedRectRadii::from_single_radius(track_height / 2.0),
            ).to_path(0.1),
        );

        // Draw filled track (up to thumb position) using primary-dark
        let filled_width = (layout_node.layout.size.width * value) as f64;
        if filled_width > 0.0 {
            graphics.fill(
                Fill::NonZero,
                Affine::default(),
                &Brush::Solid(filled_track_color),
                None,
                &RoundedRect::from_rect(
                    Rect::new(
                        layout_node.layout.location.x as f64,
                        track_top,
                        layout_node.layout.location.x as f64 + filled_width,
                        track_bottom,
                    ),
                    RoundedRectRadii::from_single_radius(track_height / 2.0),
                ).to_path(0.1),
            )
        }

        // Draw rectangular thumb (old-style UI slider)
        let thumb_width = 12.0;
        let thumb_height = 16.0;
        let thumb_x = layout_node.layout.location.x as f64 + (layout_node.layout.size.width * value) as f64 - thumb_width / 2.0;
        let thumb_y = track_center_y - thumb_height / 2.0;

        graphics.fill(
            Fill::NonZero,
            Affine::default(),
            &thumb_brush,
            None,
            &RoundedRect::from_rect(
                Rect::new(
                    thumb_x,
                    thumb_y,
                    thumb_x + thumb_width,
                    thumb_y + thumb_height,
                ),
                RoundedRectRadii::from_single_radius(2.0), // Slightly rounded corners
            ).to_path(0.1),
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
