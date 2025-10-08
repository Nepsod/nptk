use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{Dimension, LayoutStyle, LengthPercentageAuto};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::Theme;
use nptk::theme::theme::dark::DarkTheme;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::theme::config::{ThemeConfig, ThemeSource};
use nptk::theme::theme_resolver::SelfContainedThemeResolver;
use nptk::theme::id::WidgetId;
use nptk::theme::globals::Globals;
use nptk::color::Color;
use nptk::widgets::container::Container;
use nptk::widgets::text::Text;

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

struct ThemeConfigApp;

impl Application for ThemeConfigApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        // Create the UI content
        Container::new(vec![
            // Title
            Box::new(Text::new("Theme Configuration Demo".to_string())
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::percent(1.0), Dimension::length(40.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::length(0.0),
                        top: LengthPercentageAuto::length(20.0),
                        bottom: LengthPercentageAuto::length(20.0),
                    },
                    ..Default::default()
                })),
            
            // Theme information
            Box::new(Text::new("Theme configuration system is working!".to_string())
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::length(0.0),
                        top: LengthPercentageAuto::length(10.0),
                        bottom: LengthPercentageAuto::length(10.0),
                    },
                    ..Default::default()
                })),
            
            // Environment variable information
            Box::new(Text::new("Set NPTK_THEME environment variable to change theme".to_string())
                .with_layout_style(LayoutStyle {
                    size: Vector2::new(Dimension::percent(1.0), Dimension::length(30.0)),
                    margin: nptk::core::layout::Rect::<LengthPercentageAuto> {
                        left: LengthPercentageAuto::length(0.0),
                        right: LengthPercentageAuto::length(0.0),
                        top: LengthPercentageAuto::length(10.0),
                        bottom: LengthPercentageAuto::length(10.0),
                    },
                    ..Default::default()
                })),
        ])
        .with_layout_style(LayoutStyle {
            size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
            padding: nptk::core::layout::Rect::<nptk::core::layout::LengthPercentage> {
                left: nptk::core::layout::LengthPercentage::length(20.0),
                right: nptk::core::layout::LengthPercentage::length(20.0),
                top: nptk::core::layout::LengthPercentage::length(20.0),
                bottom: nptk::core::layout::LengthPercentage::length(20.0),
            },
            gap: nptk::math::Vector2::new(
                nptk::core::layout::LengthPercentage::length(10.0),
                nptk::core::layout::LengthPercentage::length(10.0),
            ),
            ..Default::default()
        })
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        // Load theme configuration and create the appropriate theme
        let config = ThemeConfig::from_env_or_default();
        let theme = ConfigurableTheme::from_config(&config);
        
        MayConfig {
            theme,
            ..Default::default()
        }
    }
}

fn main() {
    // Print environment variable information
    println!("Theme Configuration Demo");
    println!("========================");
    println!("Set the following environment variables to configure the theme:");
    println!("  NPTK_THEME=light     # Use light theme");
    println!("  NPTK_THEME=dark      # Use dark theme");
    println!("  NPTK_THEME=custom:my-theme  # Use custom theme");
    println!("  NPTK_THEME_FALLBACK=light   # Set fallback theme");
    println!();
    
    if let Ok(theme_env) = std::env::var("NPTK_THEME") {
        println!("Current NPTK_THEME: {}", theme_env);
    } else {
        println!("NPTK_THEME not set, using default theme");
    }
    
    if let Ok(fallback_env) = std::env::var("NPTK_THEME_FALLBACK") {
        println!("Current NPTK_THEME_FALLBACK: {}", fallback_env);
    } else {
        println!("NPTK_THEME_FALLBACK not set, using default fallback");
    }
    
    println!();
    println!("Starting application...");
    
    // Demonstrate theme configuration
    let config = ThemeConfig::from_env_or_default();
    println!("Theme configuration loaded:");
    println!("  Default theme: {:?}", config.default_theme);
    println!("  Fallback theme: {:?}", config.fallback_theme);
    
    // Demonstrate theme resolver
    let resolver = SelfContainedThemeResolver::new();
    let theme_name = match &config.default_theme {
        nptk::theme::config::ThemeSource::Light => "light",
        nptk::theme::config::ThemeSource::Dark => "dark",
        nptk::theme::config::ThemeSource::Custom(name) => name,
        nptk::theme::config::ThemeSource::File(path) => path,
    };
    match resolver.resolve_theme(theme_name) {
        Ok(_) => println!("  Successfully resolved theme: {}", theme_name),
        Err(e) => println!("  Failed to resolve theme {}: {}", theme_name, e),
    }
    
    println!();
    println!("Running GUI application...");
    
    // Run the application
    let app = ThemeConfigApp;
    app.run(());
}