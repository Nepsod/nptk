use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::signal::eval::EvalSignal;
use nptk::core::signal::Signal;
use nptk::core::widget::Widget;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::gesture_detector::GestureDetector;
use nptk::widgets::text::Text;

struct MyApp;

impl Application for MyApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        GestureDetector::new(Text::new("Gesture Detector".to_string()))
            .with_on_hover(
                EvalSignal::new(move || {
                    println!("Hovered");
                    Update::DRAW
                })
                .hook(&context)
                .maybe(),
            )
            .with_on_release(
                EvalSignal::new(move || {
                    println!("Release");
                    Update::DRAW
                })
                .hook(&context)
                .maybe(),
            )
            .with_on_press(
                EvalSignal::new(move || {
                    println!("Press");
                    Update::DRAW
                })
                .hook(&context)
                .maybe(),
            )
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    MyApp.run(())
}
