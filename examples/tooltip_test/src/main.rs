use nptk::prelude::*;
use nptk::core::layout::{FlexDirection, LayoutStyle, LengthPercentage, Dimension};
use nptk::math::Vector2;

struct TooltipTestApp;

impl Application for TooltipTestApp {
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Container::new(vec![
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
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            gap: Vector2::new(
                LengthPercentage::length(0.0),
                LengthPercentage::length(10.0),
            ),
            padding: nptk::core::layout::Rect::<LengthPercentage> {
                left: LengthPercentage::length(10.0),
                right: LengthPercentage::length(10.0),
                top: LengthPercentage::length(10.0),
                bottom: LengthPercentage::length(10.0),
            },
            ..Default::default()
        })
    }
}

fn main() {
    println!("DEBUG: Starting tooltip test application");
    TooltipTestApp.run(())
}