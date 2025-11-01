use nptk_core::{
    app::{context::AppContext, update::Update, Application},
    config::MayConfig,
    layout::{Dimension, LayoutStyle},
    widget::{Widget, WidgetLayoutExt},
};
use nptk_theme::theme::Theme;
use nptk_theme::theme::celeste::CelesteTheme;
use nptk_theme::theme::dark::DarkTheme;
use nptk_theme::config::{ThemeConfig, ThemeSource};
use nptk_theme::id::WidgetId;
use nptk_theme::globals::Globals;
use nptk_core::vg::peniko::Color;
use nptk_widgets::{
    tabs_container::{TabItem, TabsContainer, TabPosition},
    text::Text,
};

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
struct TabsApp;

impl Application for TabsApp {
    type Theme = ConfigurableTheme;
    type State = ();

    fn config(&self) -> nptk_core::config::MayConfig<Self::Theme> {
        // Load theme configuration and create the appropriate theme
        let config = ThemeConfig::from_env_or_default();
        let theme = ConfigurableTheme::from_config(&config);
        
        MayConfig {
            theme,
            ..Default::default()
        }
    }

    fn build(_context: AppContext, _config: Self::State) -> impl Widget {
        // Create tab content with more descriptive text
        let tab1_content = Text::new("Welcome to Tab 1! This content appears in the content area below the tabs. The tab bar is at the top and the content is properly separated.".to_string());
        let tab2_content = Text::new("This is Tab 2's content. Notice how the content changes when you click different tabs, and it's properly contained in the content area.".to_string());
        let tab3_content = Text::new("Tab 3 content is displayed here. This tab has a close button (X) that you can click. The content area is clearly separated from the tab bar.".to_string());
        let tab4_content = Text::new("Tab 4 demonstrates that you can have multiple tabs. Try changing the tab position to Bottom, Left, or Right to see different layouts!".to_string());

        // Create tabs
        let tab1 = TabItem::new("tab1", "Home", tab1_content);
        let tab2 = TabItem::new("tab2", "Settings", tab2_content);
        let tab3 = TabItem::new("tab3", "Help", tab3_content)
            .with_close_callback(|| {
                println!("Help tab close button clicked!");
                Update::empty()
            });
        let tab4 = TabItem::new("tab4", "About", tab4_content);

        // Create tabs container - try different positions!
        TabsContainer::new()
            .with_tab(tab1)
            .with_tab(tab2)
            .with_tab(tab3)
            .with_tab(tab4)
            .with_position(TabPosition::Bottom) // Try: Top, Bottom, Left, Right
            .with_tab_size(40.0)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            })
    }
}

fn main() {
    TabsApp.run(())
}
