use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, RoundedRect, RoundedRectRadii, Stroke};
use nptk_core::vg::peniko::{Brush, Fill};
use nptk_core::vg::Scene;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;

/// A progress bar widget to display progress from `0.0` to `1.0`.
///
/// ### Theming
/// You can style the progress bar using following properties:
/// - `color` - The background color of the progress bar.
/// - `color_progress` - The color of the progress fill.
/// - `color_border` - The border color of the progress bar (optional).
///
/// ### Features
/// - Determinate progress (0.0 to 1.0)
/// - Indeterminate progress (animated)
/// - Customizable colors and styling
/// - Smooth animations
pub struct Progress {
    layout_style: MaybeSignal<LayoutStyle>,
    value: MaybeSignal<f32>,
    indeterminate: MaybeSignal<bool>,
    animation_time: f32,
}

impl Progress {
    /// Create a new Progress widget with a value signal.
    /// 
    /// # Arguments
    /// * `value` - Progress value between 0.0 and 1.0
    pub fn new(value: impl Into<MaybeSignal<f32>>) -> Self {
        Self {
            layout_style: LayoutStyle {
                size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(20.0)),
                margin: layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(10.0),
                    right: LengthPercentageAuto::length(10.0),
                    top: LengthPercentageAuto::length(5.0),
                    bottom: LengthPercentageAuto::length(5.0),
                },
                ..Default::default()
            }
            .into(),
            value: value.into(),
            indeterminate: MaybeSignal::value(false),
            animation_time: 0.0,
        }
    }

    /// Sets the layout style of the progress bar and returns itself.
    pub fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = layout_style.into();
        self
    }

    /// Sets whether the progress bar should show indeterminate (animated) progress.
    pub fn with_indeterminate(mut self, indeterminate: impl Into<MaybeSignal<bool>>) -> Self {
        self.indeterminate = indeterminate.into();
        self
    }
}

impl WidgetLayoutExt for Progress {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for Progress {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        let width = layout.layout.size.width as f64;
        let height = layout.layout.size.height as f64;
        let x = layout.layout.location.x as f64;
        let y = layout.layout.location.y as f64;

        // Get theme colors
        let (background_color, progress_color, border_color) = if let Some(style) = theme.of(self.widget_id()) {
            (
                style.get_color("color").unwrap_or_else(|| nptk_core::vg::peniko::Color::from_rgb8(220, 220, 220)),
                style.get_color("color_progress").unwrap_or_else(|| nptk_core::vg::peniko::Color::from_rgb8(100, 150, 255)),
                style.get_color("color_border").unwrap_or_else(|| nptk_core::vg::peniko::Color::from_rgb8(180, 180, 180)),
            )
        } else {
            (
                nptk_core::vg::peniko::Color::from_rgb8(220, 220, 220),
                nptk_core::vg::peniko::Color::from_rgb8(100, 150, 255),
                nptk_core::vg::peniko::Color::from_rgb8(180, 180, 180),
            )
        };

        // Draw background
        let background_rect = RoundedRect::new(
            x,
            y,
            x + width,
            y + height,
            RoundedRectRadii::from_single_radius(height / 4.0),
        );

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            background_color,
            None,
            &background_rect,
        );

        // Draw border
        let border_rect = RoundedRect::new(
            x,
            y,
            x + width,
            y + height,
            RoundedRectRadii::from_single_radius(height / 4.0),
        );
        
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(border_color),
            None,
            &border_rect,
        );

        // Draw progress fill
        if *self.indeterminate.get() {
            // Indeterminate mode: animated progress
            let progress_width = width * 0.3; // 30% of total width
            let progress_x = x + (self.animation_time as f64 * (width + progress_width) - progress_width).max(0.0).min(width - progress_width);
            
            let progress_rect = RoundedRect::new(
                progress_x,
                y + 1.0, // Small margin from border
                progress_x + progress_width,
                y + height - 1.0,
                RoundedRectRadii::from_single_radius((height - 2.0) / 4.0),
            );

            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                progress_color,
                None,
                &progress_rect,
            );
        } else {
            // Determinate mode: show actual progress
            let progress_value = (*self.value.get()).clamp(0.0, 1.0);
            let progress_width = width * progress_value as f64;
            
            if progress_width > 0.0 {
                let progress_rect = RoundedRect::new(
                    x + 1.0, // Small margin from border
                    y + 1.0,
                    x + progress_width - 1.0,
                    y + height - 1.0,
                    RoundedRectRadii::from_single_radius((height - 2.0) / 4.0),
                );

                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    progress_color,
                    None,
                    &progress_rect,
                );
            }
        }
    }

    fn update(&mut self, _layout: &LayoutNode, _context: AppContext, _info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Update animation time for indeterminate progress
        if *self.indeterminate.get() {
            // Simple time-based animation (in a real app, you'd use proper timing)
            self.animation_time += 0.016; // ~60fps
            if self.animation_time > 1.0 {
                self.animation_time = 0.0;
            }
            update |= Update::DRAW | Update::EVAL; // Request both draw and eval to ensure continuous updates
        }

        update
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![],
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Progress")
    }
}
