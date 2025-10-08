use nptk::core::app::context::AppContext;
use nptk::core::app::Application;
use nptk::core::config::MayConfig;
use nptk::core::layout::{AlignItems, Dimension, FlexDirection, LayoutStyle};
use nptk::core::widget::{Widget, WidgetLayoutExt};
use nptk::math::Vector2;
use nptk::theme::theme::Theme;
use nptk::theme::theme::dark::DarkTheme;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::theme::config::{ThemeConfig, ThemeSource};
use nptk::theme::id::WidgetId;
use nptk::theme::globals::Globals;
use nptk::core::vg::peniko::Color;
use nptk::widgets::container::Container;
use nptk::widgets::secret_input::SecretInput;
use nptk::widgets::text::Text;
use nptk::widgets::text_input::TextInput;

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
struct TextInputApp;

impl Application for TextInputApp {
    type Theme = ConfigurableTheme;
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
    TextInputApp.run(())
}
