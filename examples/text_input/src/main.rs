use nptk::prelude::*;

struct TextInputApp;

impl Application for TextInputApp {
    type State = ();

    fn build(_context: AppContext, _: Self::State) -> impl Widget {
        Container::new(vec![
            Box::new(Text::new("Text Input Demo".to_string())),
            Box::new(Text::new(
                "Click on the input fields and start typing".to_string(),
            )),
            Box::new(Text::new("Regular Text Input:".to_string())),
            Box::new(TextInput::new().with_placeholder("Enter some text here...".to_string())),
            Box::new(Text::new("Password Input:".to_string())),
            Box::new(SecretInput::new().with_placeholder("Enter your password...".to_string())),
            Box::new(Text::new("Another Text Input:".to_string())),
            Box::new(TextInput::new().with_placeholder("More text input...".to_string())),
            Box::new(Text::new(
                "Use Tab to navigate, CTRL+A/C/X/V for shortcuts".to_string(),
            )),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::<Dimension>::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            gap: Vector2::new(
                nptk::core::layout::LengthPercentage::length(0.0),
                nptk::core::layout::LengthPercentage::length(20.0),
            ),
            ..Default::default()
        })
    }
}

fn main() {
    TextInputApp.run(())
}
