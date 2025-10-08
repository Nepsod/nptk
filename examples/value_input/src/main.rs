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
use nptk::widgets::text::Text;
use nptk::widgets::value_input::ValueInput;

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
struct ValueInputApp;

impl Application for ValueInputApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn build(_context: AppContext, _: Self::State) -> impl Widget {
        Container::new(vec![
            Box::new(Text::new("Value Input Demo".to_string())),
            Box::new(Text::new("Integer Input:".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(42.0)
                    .with_placeholder("Enter integer...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Decimal Input (2 places):".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(3.14)
                    .with_decimal_places(2)
                    .with_step(0.1)
                    .with_placeholder("Enter decimal...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Range-constrained Input (0-100):".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(50.0)
                    .with_min(0.0)
                    .with_max(100.0)
                    .with_step(5.0)
                    .with_placeholder("Enter 0-100...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Negative Values Allowed:".to_string())),
            Box::new(
                ValueInput::new()
                    .with_value(-25.0)
                    .with_negative(true)
                    .with_placeholder("Enter any number...".to_string())
                    .with_layout_style(LayoutStyle {
                        size: Vector2::<Dimension>::new(Dimension::length(200.0), Dimension::length(40.0)),
                        ..Default::default()
                    })
            ),
            Box::new(Text::new("Use Up/Down arrows to increment/decrement".to_string())),
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
    ValueInputApp.run(())
}
