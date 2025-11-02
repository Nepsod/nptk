use nptk::color::color::palette;
use nptk::color::kurbo::{Affine, Circle, Point, Stroke};
use nptk::color::Brush;
use nptk::prelude::*;
use nptk::widgets::canvas::Canvas;

struct MyApp;

impl Application for MyApp {
    type Theme = SystemTheme;
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
}

fn main() {
    MyApp.run(())
}
