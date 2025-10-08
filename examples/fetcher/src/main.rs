use nptk::core::app::context::AppContext;
use nptk::core::app::update::Update;
use nptk::core::app::Application;
use nptk::core::config::{MayConfig, TasksConfig};
use nptk::core::widget::Widget;
use nptk::theme::theme::Theme;
use nptk::theme::theme::dark::DarkTheme;
use nptk::theme::theme::celeste::CelesteTheme;
use nptk::theme::config::{ThemeConfig, ThemeSource};
use nptk::theme::id::WidgetId;
use nptk::theme::globals::Globals;
use nptk::core::vg::peniko::Color;
use nptk::widgets::fetcher::WidgetFetcher;
use nptk::widgets::text::Text;
use serde::Deserialize;

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
struct MyApp;

impl Application for MyApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn build(_: AppContext, _: Self::State) -> impl Widget {
        WidgetFetcher::new(get_random_quote(), Update::DRAW, |data| {
            if let Some(data) = data {
                Text::new(format!(" \"{}\" \n - {}", data.quote, data.author))
            } else {
                Text::new(" Loading Quote...".to_string())
            }
        })
    }

    fn config(&self) -> MayConfig<Self::Theme> {
        MayConfig {
            tasks: Some(TasksConfig::default()),
            ..Default::default()
        }
    }
}

fn main() {
    MyApp.run(())
}

#[derive(Deserialize)]
struct Quote {
    quote: String,
    author: String,
}

async fn get_random_quote() -> Quote {
    surf::get("https://dummyjson.com/quotes/random")
        .await
        .expect("Failed to fetch quote")
        .body_json::<Quote>()
        .await
        .expect("Failed to parse quote")
}
