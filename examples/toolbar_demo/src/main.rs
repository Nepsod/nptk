use nptk::prelude::*;

struct MyApp;

impl Application for MyApp {
    type Theme = SystemTheme;
    type State = ();

    fn build(_context: AppContext, _: Self::State) -> impl Widget {
        Container::new(vec![
            Box::new(
                Toolbar::new()
                    .with_child(ToolbarButton::new(Text::new("New".to_string()).with_font_size(24.0).with_font(MaybeSignal::value(Some("WenQuanYi Micro Hei".to_string())))))
                    .with_child(ToolbarButton::new(Text::new("Open".to_string()).with_font_size(24.0)))
                    .with_separator()
                    .with_child(ToolbarButton::new(Text::new("Save".to_string()).with_font_size(24.0)))
                    .with_spacer()
                    .with_child(ToolbarButton::new(Text::new("Help❔".to_string()).with_font_size(24.0).with_font(MaybeSignal::value(Some("Noto Color Emoji".to_string())))))
            )
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::<Dimension>::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
    }
}

fn main() {
    MyApp.run(())
}
