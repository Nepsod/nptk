use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::future::FutureSignal;
use nptk_core::signal::Signal;
use nptk_core::vgi::Graphics;
use nptk_core::widget::Widget;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::future::Future;

/// Widget builder to fetch data from an asynchronous task.
///
/// ### Async + UI
/// The [WidgetFetcher] uses the application context to spawn asynchronous tasks on a global runner and construct a widget based on the result of the task.
pub struct WidgetFetcher<T: Send + Sync + Clone + 'static, W: Widget, F: Fn(Option<T>) -> W> {
    result: FutureSignal<T>,
    render: F,
    widget: Option<W>,
    update: Update,
    has_rendered_result: bool,
}

impl<T: Send + Sync + Clone + 'static, W: Widget, F: Fn(Option<T>) -> W> WidgetFetcher<T, W, F> {
    /// Creates a new [WidgetFetcher] with parameters:
    /// - `future`: The future to execute.
    /// - `update`: The update to trigger when the data is updated (from loading to done).
    /// - `render`: The function to render the widget. The first parameter is the result of the future and the second parameter is the mutable app state.
    pub fn new<Fut>(future: Fut, update: Update, render: F) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
    {
        let signal = FutureSignal::new(future);

        Self {
            result: signal,
            render,
            widget: None,
            update,
            has_rendered_result: false,
        }
    }
}

impl<T: Send + Sync + Clone + 'static, W: Widget, F: Fn(Option<T>) -> W> Widget for WidgetFetcher<T, W, F> {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if let Some(widget) = &mut self.widget {
            widget.render(graphics, theme, layout_node, info, context)
        }
    }

    fn layout_style(&self) -> StyleNode {
        if let Some(widget) = &self.widget {
            widget.layout_style()
        } else {
            StyleNode {
                style: LayoutStyle::default(),
                children: Vec::new(),
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        // Register notify callback to trigger update when future completes
        let update_type = self.update;
        let update_manager = context.update();
        self.result.on_complete(move || {
            update_manager.insert(update_type);
        });
        
        let mut update = Update::empty();
        let async_state = self.result.get();
        
        if async_state.is_ready() {
            if !self.has_rendered_result {
                if let nptk_core::signal::async_state::AsyncState::Ready(val) = &*async_state {
                    self.widget = Some((self.render)(Some(val.clone())));
                    self.has_rendered_result = true;
                    update = self.update;
                }
            }
        } else if self.widget.is_none() {
             self.widget = Some((self.render)(None));
             update = self.update;
        }

        self.widget.as_mut().unwrap().update(layout, context, info) | update
    }

    fn widget_id(&self) -> WidgetId {
        if let Some(widget) = &self.widget {
            widget.widget_id()
        } else {
            WidgetId::new("nptk-widgets", "WidgetFetcher")
        }
    }
}
