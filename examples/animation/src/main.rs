use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::signal::Signal;
use nptk::core::widget::Widget;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::animator::Animator;
use nptk::widgets::text::Text;
use std::time::Duration;

struct MyApp;

impl Application for MyApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let font_size = context.use_state(0.0);

        Animator::new(
            Duration::from_millis(2000),
            Text::new("Hello World!".to_string()).with_font_size(font_size.maybe()),
            move |_, f| {
                font_size.set(f * 30.0);

                Update::DRAW
            },
        )
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    MyApp.run(())
}
