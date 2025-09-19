use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::reference::Ref;
use nptk::core::signal::state::StateSignal;
use nptk::core::signal::Signal;
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::container::Container;
use nptk::widgets::slider::Slider;
use nptk::widgets::text::Text;

struct MyApp;

impl Application for MyApp {
    type Theme = CelesteTheme;
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

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }
}

fn main() {
    MyApp.run(())
}
