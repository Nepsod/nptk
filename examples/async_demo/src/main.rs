use nptk::prelude::*;
use nptk::core::signal::async_state::AsyncState;
use nptk::core::app::info::AppInfo;
use nptk::core::text_render::TextRenderContext;
use nptk::core::vgi::Graphics;
use nptk::theme::theme::Theme;
use nptk::color::Color;
use nptk::core::app::runner::MayRunner;

#[derive(Default)]
struct AppState;

struct AsyncWidget {
    result: MaybeSignal<AsyncState<String>>,
    text_ctx: TextRenderContext,
}

impl AsyncWidget {
    fn new(context: AppContext) -> Self {
        let future = async {
            // Simulate work
            println!("Starting async work...");
            // Use std::thread::sleep for demo purposes (blocks one thread in the pool)
            std::thread::sleep(std::time::Duration::from_secs(2));
            println!("Async work finished!");
            "Hello from the Future!".to_string()
        };

        let signal = context.use_future(future);

        // Convert FutureSignal to MaybeSignal
        // FutureSignal implements Signal<AsyncState<String>>
        // We need to box it to create a BoxedSignal, which MaybeSignal accepts
        let boxed_signal: Box<dyn Signal<AsyncState<String>>> = Box::new(signal);

        Self {
            result: MaybeSignal::signal(boxed_signal),
            text_ctx: TextRenderContext::new(),
        }
    }
}

impl Widget for AsyncWidget {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Draw background
        let rect = nptk::core::vg::kurbo::Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );
        
        let bg_color = theme.window_background();
        
        graphics.fill(
            nptk::core::vg::peniko::Fill::NonZero,
            nptk::core::vg::kurbo::Affine::IDENTITY,
            &nptk::core::vg::peniko::Brush::Solid(bg_color),
            None,
            &nptk::core::vgi::shape_to_path(&rect),
        );

        let text = match self.result.get().as_ref() {
            AsyncState::Loading => "Loading... (Wait 2s)".to_string(),
            AsyncState::Ready(val) => format!("Result: {}", val),
            AsyncState::Error(err) => format!("Error: {}", err),
        };

        let color = match self.result.get().as_ref() {
            AsyncState::Loading => Color::from_rgb8(100, 100, 100),
            AsyncState::Ready(_) => Color::from_rgb8(0, 150, 0),
            AsyncState::Error(_) => Color::from_rgb8(200, 0, 0),
        };

        // Draw text
        self.text_ctx.render_text(
            &mut info.font_context,
            graphics,
            &text,
            None,
            32.0,
            nptk::core::vg::peniko::Brush::Solid(color),
            nptk::core::vg::kurbo::Affine::translate((
                layout.layout.location.x as f64 + 20.0,
                layout.layout.location.y as f64 + 20.0,
            )),
            true,
            None,
        );
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: LayoutStyle {
                size: nptk::math::Vector2::new(
                    Dimension::percent(1.0),
                    Dimension::percent(1.0),
                ),
                ..Default::default()
            },
            children: vec![],
        }
    }

    fn update(
        &mut self,
        _layout: &LayoutNode,
        _context: AppContext,
        _info: &mut AppInfo,
    ) -> Update {
        // When signal changes (via Eval update), we need to redraw
        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("async_demo", "AsyncWidget")
    }
}

fn main() {
    let config = nptk::core::config::MayConfig {
        tasks: Some(nptk::core::config::TasksConfig::default()),
        window: nptk::core::config::WindowConfig {
            title: "Async Demo".to_string(),
            size: nptk::math::Vector2::new(400.0, 300.0),
            ..Default::default()
        },
        ..Default::default()
    };
    
    MayRunner::new(config).run(
        AppState::default(),
        move |context, _state| {
            AsyncWidget::new(context)
        },
        nptk::core::plugin::PluginManager::new(),
    );
}
