use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutContext, LayoutNode, StyleNode};
use nptk_core::vgi::Graphics;
use nptk_core::widget::Widget;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;
use std::time::{Duration, Instant};

/// A widget that animates another widget using an animation function.
pub struct Animator<W: Widget, A: Fn(&mut W, f32) -> Update + Send + Sync> {
    start: Instant,
    duration: Duration,
    widget: W,
    animation: A,
}

impl<W: Widget, A: Fn(&mut W, f32) -> Update + Send + Sync> Animator<W, A> {
    /// Creates a new animator widget with the given duration, child widget and animation function.
    ///
    /// The animation function is called with a value between `0.0` and `1.0` based on the elapsed time since the start of the animation
    /// and the total duration of the animation.
    pub fn new(duration: Duration, widget: W, animation: A) -> Self {
        Self {
            start: Instant::now(),
            duration,
            widget,
            animation,
        }
    }
}

#[async_trait(?Send)]
impl<W: Widget, A: Fn(&mut W, f32) -> Update + Send + Sync> Widget for Animator<W, A> {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        self.widget
            .render(graphics, layout_node, info, context);
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        self.widget.layout_style(context)
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let elapsed = self.start.elapsed();

        let mut update = self.widget.update(layout, context, info).await;

        if elapsed < self.duration {
            let f = elapsed.as_secs_f32() / self.duration.as_secs_f32();

            update.insert((self.animation)(&mut self.widget, f));
        }

        update
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Animator")
    }
}
