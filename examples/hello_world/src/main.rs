use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::widget::Widget;
use nptk::theme::theme::Theme;
use nptk::theme::theme::dark::DarkTheme;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::theme::config::{ThemeConfig, ThemeSource};
use nptk::theme::id::WidgetId;
use nptk::theme::style::{DefaultStyles, Style};
use nptk::theme::globals::Globals;
use nptk::core::vg::peniko::Color;
use nptk::widgets::text::Text;

/// A wrapper theme that can switch between different themes
#[derive(Clone)]
pub enum ConfigurableTheme {
    Light(CelesteTheme),
    Dark(DarkTheme),
}

impl Theme for ConfigurableTheme {
    fn of(&self, id: WidgetId) -> Option<Style> {
        match self {
            ConfigurableTheme::Light(theme) => theme.of(id),
            ConfigurableTheme::Dark(theme) => theme.of(id),
        }
    }

    fn defaults(&self) -> DefaultStyles {
        match self {
            ConfigurableTheme::Light(theme) => theme.defaults(),
            ConfigurableTheme::Dark(theme) => theme.defaults(),
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
}

// ThemeRenderer is automatically implemented via blanket impl for all Theme types

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

struct MyApp;

impl Application for MyApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        Text::new("Hello World! Unicode: ≠, ←, ↓, →, ≤\n\nMultilingual Test:\n中文 (Chinese)\n한국어 (Korean)\nالعربية (Arabic)\nहिन्दी (Hindi)\nবাংলা (Bengali)\nதமிழ் (Tamil)\nമലയാളം (Malayalam)\nગુજરાતી (Gujarati)\nਪੰਜਾਬੀ (Punjabi)\nاردو (Urdu)\nفارسی (Persian)\nעברית (Hebrew)\nРусский (Russian)\nΕλληνικά (Greek)\n日本語 (Japanese)".to_string())
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
    println!("DEBUG: Starting hello_world application");
    MyApp.run(())
}
