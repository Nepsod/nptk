// SPDX-License-Identifier: LGPL-3.0-only
use crate::theme_rendering::render_progress_with_theme;
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout;
use nptk_core::layout::{Dimension, LayoutContext, LayoutNode, LayoutStyle, LengthPercentageAuto, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use async_trait::async_trait;

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

#[async_trait(?Send)]
impl Widget for Progress {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        _info: &mut AppInfo,
        context: AppContext,
    ) {
        // Use palette-based rendering
        let palette = context.palette();
        render_progress_with_theme(
            &palette,
            *self.value.get(),
            *self.indeterminate.get(),
            self.animation_time,
            layout,
            graphics,
        );
    }

    async fn update(
        &mut self,
        _layout: &LayoutNode,
        _context: AppContext,
        _info: &mut AppInfo,
    ) -> Update {
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

    fn layout_style(&self, _context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![],
            measure_func: None,
        }
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Progress")
    }
}
