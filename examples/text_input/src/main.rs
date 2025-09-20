use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::container::Container;
use nptk::widgets::secret_input::SecretInput;
use nptk::widgets::text::Text;
use nptk::widgets::text_input::TextInput;

struct TextInputApp;

impl Application for TextInputApp {
    type Theme = CelesteTheme;
    type State = ();

    fn build(_context: AppContext, _: Self::State) -> impl Widget {
        Container::new(vec![
            Box::new(Text::new("Text Input Demo".to_string())),
            Box::new(Text::new("Click on the input fields and start typing".to_string())),
            Box::new(Text::new("Regular Text Input:".to_string())),
            Box::new(
                TextInput::new()
                    .with_placeholder("Enter some text here...".to_string())
            ),
            Box::new(Text::new("Password Input:".to_string())),
            Box::new(
                SecretInput::new()
                    .with_placeholder("Enter your password...".to_string())
            ),
            Box::new(Text::new("Another Text Input:".to_string())),
            Box::new(
                TextInput::new()
                    .with_placeholder("More text input...".to_string())
            ),
            Box::new(Text::new("Use Tab to navigate, CTRL+A/C/X/V for shortcuts".to_string())),
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
    TextInputApp.run(())
}
