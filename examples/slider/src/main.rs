use nptk::prelude::*;

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let value = context.use_signal(StateSignal::new(0.0f32));

        Container::new(vec![
            Box::new(Slider::new(value.maybe())),
            Box::new(Text::new(value.map(|i| Ref::Owned(i.to_string())))),
        ])
        .with_layout_style(LayoutStyle {
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
