use nptk::color::color::palette;
use nptk::core::vg::kurbo::{Affine, Circle, Point, Shape, Stroke};
use nptk::color::Brush;
use nptk::prelude::*;
use nptk::widgets::canvas::Canvas;

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Canvas::new(|graphics, _theme, _layout, _info| {
            graphics.stroke(
                &Stroke::new(10.0),
                Affine::default(),
                &Brush::Solid(palette::css::GREEN),
                None,
                &Circle::new(Point::new(100.0, 100.0), 50.0).to_path(0.1),
            );
        })
    }
}

fn main() {
    MyApp.run(())
}
