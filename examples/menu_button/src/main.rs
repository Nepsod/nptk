use nptk::prelude::*;

struct MenuButtonApp;

impl Application for MenuButtonApp {
    type Theme = SystemTheme;
    type State = ();

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
            MenuItem::new("exit", "Exit").with_on_activate(|| {
                println!("Exit clicked!");
                Update::empty()
            }),
        ];

        Container::new(vec![
            Box::new(Text::new("MenuButton Demo".to_string())),
            Box::new(Text::new(
                "Click the button below to see the popup menu:".to_string(),
            )),
            Box::new(Text::new(
                "Use Tab/Shift+Tab to navigate, Space/Enter to open menu".to_string(),
            )),
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
                    }),
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
    // Print environment variable information
    println!("MenuButton Demo");
    println!("===============");
    println!("Set the following environment variables to configure the theme:");
    println!("  NPTK_THEME=light     # Use light theme");
    println!("  NPTK_THEME=dark      # Use dark theme");
    println!();

    if let Ok(theme_env) = std::env::var("NPTK_THEME") {
        println!("Current NPTK_THEME: {}", theme_env);
    } else {
        println!("NPTK_THEME not set, using default theme");
    }

    println!();
    println!("Starting application...");

    // Demonstrate theme configuration
    let config = ThemeConfig::from_env_or_default();
    println!("Theme configuration loaded:");
    println!("  Default theme: {:?}", config.default_theme);
    println!("  Fallback theme: {:?}", config.fallback_theme);

    println!();
    println!("Running GUI application...");

    // Run the application
    let app = MenuButtonApp;
    app.run(());
}
