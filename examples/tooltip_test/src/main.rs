use nptk::prelude::*;

struct TooltipTestApp;

impl Application for TooltipTestApp {
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Container::new(
            VStack::new(vec![
                Box::new(Text::new("Tooltip Test - Hover over the buttons below".to_string())),
                Box::new(
                    Button::new(Text::new("Button with Tooltip".to_string()))
                        .with_tooltip("This is a test tooltip!")
                ),
                Box::new(
                    Button::new(Text::new("Another Button".to_string()))
                        .with_tooltip("Another tooltip with longer text to test positioning")
                ),
            ])
        )
    }
}

fn main() {
    println!("DEBUG: Starting tooltip test application");
    TooltipTestApp.run(())
}