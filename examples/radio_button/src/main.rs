use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::container::Container;
use nptk::widgets::text::Text;
use nptk::widgets::radio_button::RadioButton;

struct RadioButtonApp;

impl Application for RadioButtonApp {
    type Theme = CelesteTheme;
    type State = ();

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        Container::new(vec![
            Box::new(Text::new("Radio Button Demo".to_string())),
            Box::new(Text::new("Choose your favorite color:".to_string())),
            
            Box::new(RadioButton::new("Red".to_string(), "color".to_string())),
            Box::new(RadioButton::new("Green".to_string(), "color".to_string())),
            Box::new(RadioButton::new("Blue".to_string(), "color".to_string())),
            Box::new(RadioButton::new("Yellow".to_string(), "color".to_string()).with_disabled(true)),
            
            Box::new(Text::new("Use Tab/Shift+Tab to navigate, Space/Enter to select".to_string())),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            ..Default::default()
        })
    }
}

fn main() {
    RadioButtonApp.run(())
}
