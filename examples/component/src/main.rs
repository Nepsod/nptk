use crate::counter::Counter;
use nptk::prelude::*;

mod counter;

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let counter = context.use_signal(StateSignal::new(0));

        Counter::new(counter).with_layout_style(LayoutStyle {
            size: Vector2::<Dimension>::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            ..Default::default()
        })
    }
}

fn main() {
    MyApp.run(())
}
