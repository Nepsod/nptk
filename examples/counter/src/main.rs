use nptk::prelude::*;

struct MyApp;

impl Application for MyApp {
    type State = ();

    fn build(context: AppContext, _: Self::State) -> impl Widget {
        let counter = context.use_signal(StateSignal::new(0));

        Container::new(vec![
            {
                let counter = counter.clone();

                Box::new(
                    Button::new(Text::new("Increase".to_string())).with_on_pressed(
                        EvalSignal::new(move || {
                            counter.set(*counter.get() + 1);

                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    ),
                )
            },
            {
                let counter = counter.clone();

                Box::new(
                    Button::new(Text::new("Decrease".to_string())).with_on_pressed(
                        EvalSignal::new(move || {
                            counter.set(*counter.get() - 1);

                            Update::DRAW
                        })
                        .hook(&context)
                        .maybe(),
                    ),
                )
            },
            Box::new(Text::new(counter.map(|i| Ref::Owned(i.to_string())))),
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
