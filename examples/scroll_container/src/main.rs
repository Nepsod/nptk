use nptk_core::app::context::AppContext;
use nptk_core::app::Application;
use nptk_core::config::MayConfig;
use nptk_core::layout::{Dimension, LayoutStyle};
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::theme::celeste::CelesteTheme;
use nptk_widgets::scroll_container::{ScrollContainer, ScrollDirection, VerticalScrollbarPosition};
use nptk_widgets::text::Text;
use nalgebra::Vector2;

struct ScrollContainerApp;

impl Application for ScrollContainerApp {
    type Theme = CelesteTheme;
    type State = ();

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }

    fn build(context: AppContext, _config: Self::State) -> impl Widget {
        let long_text = (0..100)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let text_content = Text::new(long_text).with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::length(800.0), Dimension::auto()),
            ..Default::default()
        });

        let mut scroll_container = ScrollContainer::new()
            .with_child(text_content)
            .with_scroll_direction(ScrollDirection::Vertical)
            .with_vertical_scrollbar_position(VerticalScrollbarPosition::Left)
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::length(400.0), Dimension::length(400.0)),
                ..Default::default()
            });

        scroll_container.init_reactive_scroll(&context);
        scroll_container
    }
}

fn main() {
    ScrollContainerApp.run(());
}