use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::app::update::Update;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle, LengthPercentage, LengthPercentageAuto};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::widgets::text::Text;
use nptk::widgets::menu_button::{MenuButton, MenuItem};
use nptk::widgets::container::Container;

struct MenuButtonApp;

impl Application for MenuButtonApp {
    type Theme = CelesteTheme;
    type State = ();

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig::default()
    }

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        let menu_items = vec![
            MenuItem::new("new", "New File")
                .with_shortcut("Ctrl+N")
                .with_on_activate(|| {
                    println!("New File clicked!");
                    Update::empty()
                }),
            MenuItem::new("open", "Open File")
                .with_shortcut("Ctrl+O")
                .with_on_activate(|| {
                    println!("Open File clicked!");
                    Update::empty()
                }),
            MenuItem::new("save", "Save")
                .with_shortcut("Ctrl+S")
                .with_on_activate(|| {
                    println!("Save clicked!");
                    Update::empty()
                }),
            MenuItem::new("separator", "---"),
            MenuItem::new("exit", "Exit")
                .with_on_activate(|| {
                    println!("Exit clicked!");
                    Update::empty()
                }),
        ];

        Container::new(vec![
            Box::new(Text::new("MenuButton Demo".to_string())),
            Box::new(Text::new("Click the button below to see the popup menu:".to_string())),
            Box::new(Text::new("Use Tab/Shift+Tab to navigate, Space/Enter to open menu".to_string())),
            Box::new(
                MenuButton::new("File")
                    .with_menu_items(menu_items)
                    .with_layout_style(LayoutStyle {
                        margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                            top: LengthPercentageAuto::length(0.0),
                            bottom: LengthPercentageAuto::length(0.0),
                            left: LengthPercentageAuto::length(40.0),
                            right: LengthPercentageAuto::length(0.0),
                        },
                        ..Default::default()
                    })
            ),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            gap: Vector2::new(
                LengthPercentage::length(0.0),
                LengthPercentage::length(10.0), // 10px vertical gap between items
            ),
            ..Default::default()
        })
    }
}

fn main() {
    MenuButtonApp.run(());
}
