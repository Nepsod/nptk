use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::container::Container;
use nptk::widgets::text::Text;
use nptk::widgets::value_input::ValueInput;

struct ValueInputApp;

impl Application for ValueInputApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(_context: AppContext, _: Self::State) -> impl Widget {
        Container::new(vec![
            Box::new(Text::new("Value Input Demo".to_string())),
            Box::new(Text::new("Integer Input:".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(42.0)
                    .with_placeholder("Enter integer...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Decimal Input (2 places):".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(3.14)
                    .with_decimal_places(2)
                    .with_step(0.1)
                    .with_placeholder("Enter decimal...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Range-constrained Input (0-100):".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(50.0)
                    .with_min(0.0)
                    .with_max(100.0)
                    .with_step(5.0)
                    .with_placeholder("Enter 0-100...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Negative Values Allowed:".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(-25.0)
                    .with_negative(true)
                    .with_placeholder("Enter any number...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Use Up/Down arrows to increment/decrement".to_string())),
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

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    ValueInputApp.run(())
}
