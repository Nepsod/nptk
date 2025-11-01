use nptk_core::app::context::AppContext;
use nptk_core::app::Application;
use nptk_core::config::MayConfig;
use nptk_core::layout::{Dimension, LayoutStyle};
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::theme::Theme;
use nptk_theme::theme::celeste::CelesteTheme;
use nptk_theme::theme::dark::DarkTheme;
use nptk_theme::config::{ThemeConfig, ThemeSource};
use nptk_theme::id::WidgetId;
use nptk_theme::globals::Globals;
use nptk_core::vg::peniko::Color;
use nptk_widgets::scroll_container::{ScrollContainer, ScrollDirection, VerticalScrollbarPosition};
use nptk_widgets::text::Text;
use nalgebra::Vector2;

/// A wrapper theme that can switch between different themes
#[derive(Clone)]
pub enum ConfigurableTheme {
    Light(CelesteTheme),
    Dark(DarkTheme),
}

impl Theme for ConfigurableTheme {
    fn get_property(&self, id: WidgetId, property: &nptk_theme::properties::ThemeProperty) -> Option<Color> {
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
struct ScrollContainerApp;

impl Application for ScrollContainerApp {
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