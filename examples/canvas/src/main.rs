use nptk::color::color::palette;
use nptk::color::kurbo::{Affine, Circle, Point, Stroke};
use nptk::color::Brush;
use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::widget::Widget;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::canvas::Canvas;

struct MyApp;

impl Application for MyApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Canvas::new(|scene, _| {
            scene.stroke(
                &Stroke::new(10.0),
                Affine::default(),
                &Brush::Solid(palette::css::GREEN),
                None,
                &Circle::new(Point::new(100.0, 100.0), 50.0),
            );
        })
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    MyApp.run(())
}
