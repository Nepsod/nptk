use nptk::prelude::*;

struct MyApp;

impl Application for MyApp {
    type Theme = SystemTheme;
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

}

fn main() {
    MyApp.run(())
}
