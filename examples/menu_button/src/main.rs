use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::app::update::Update;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle, LengthPercentage, LengthPercentageAuto};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::Theme;
use nptk::theme::theme::dark::DarkTheme;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::theme::config::{ThemeConfig, ThemeSource};
use nptk::theme::id::WidgetId;
use nptk::theme::globals::Globals;
use nptk::core::vg::peniko::Color;
use nptk::widgets::text::Text;
use nptk::widgets::menu_button::{MenuButton, MenuItem};
use nptk::widgets::container::Container;

/// A wrapper theme that can switch between different themes
#[derive(Clone)]
pub enum ConfigurableTheme {
    Light(CelesteTheme),
    Dark(DarkTheme),
}

impl Theme for ConfigurableTheme {
    fn get_property(&self, id: WidgetId, property: &nptk::theme::properties::ThemeProperty) -> Option<Color> {
        match self {
            ConfigurableTheme::Light(theme) => theme.get_property(id, property),
            ConfigurableTheme::Dark(theme) => theme.get_property(id, property),
        }
    }

    fn window_background(&self) -> Color {
        match self {
            ConfigurableTheme::Light(theme) => theme.window_background(),
            ConfigurableTheme::Dark(theme) => theme.window_background(),
        }
    }

    fn globals(&self) -> &Globals {
        match self {
            ConfigurableTheme::Light(theme) => theme.globals(),
            ConfigurableTheme::Dark(theme) => theme.globals(),
        }
    }

    fn globals_mut(&mut self) -> &mut Globals {
        match self {
            ConfigurableTheme::Light(theme) => theme.globals_mut(),
            ConfigurableTheme::Dark(theme) => theme.globals_mut(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for ConfigurableTheme {
    fn default() -> Self {
        ConfigurableTheme::Dark(DarkTheme::new())
    }
}

impl ConfigurableTheme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        match &config.default_theme {
            ThemeSource::Light => ConfigurableTheme::Light(CelesteTheme::light()),
            ThemeSource::Dark => ConfigurableTheme::Dark(DarkTheme::new()),
            _ => ConfigurableTheme::Dark(DarkTheme::new()), // Default fallback
        }
    }
}

struct MenuButtonApp;

impl Application for MenuButtonApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn config(&self) -> MayConfig<Self::Theme> {
        // Load theme configuration and create the appropriate theme
        let config = ThemeConfig::from_env_or_default();
        let theme = ConfigurableTheme::from_config(&config);
        
        MayConfig {
            theme,
            ..Default::default()
        }
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
