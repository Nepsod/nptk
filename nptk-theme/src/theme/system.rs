use peniko::Color;

use crate::config::{ThemeConfig, ThemeSource};
use crate::globals::Globals;
use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::{celeste::CelesteTheme, dark::DarkTheme, sweet::SweetTheme, Theme};
use std::any::Any;

/// A theme that automatically selects between light, dark, and sweet variants based on configuration.
///
/// This theme reads from environment variables or configuration files to automatically
/// select the appropriate built-in theme (light, dark, or sweet).
///
/// # Usage
///
/// ```rust
/// use nptk_theme::theme::system::SystemTheme;
///
/// // Create a system theme that respects user preferences
/// let theme = SystemTheme::default();
///
/// // Or create from a specific configuration
/// let config = ThemeConfig::from_env_or_default();
/// let theme = SystemTheme::from_config(&config);
/// ```
#[derive(Clone)]
pub enum SystemTheme {
    /// Dark theme variant.
    Dark(DarkTheme),
    /// Light theme variant (Celeste).
    Light(CelesteTheme),
    /// Sweet theme variant (modern dark with vibrant accents).
    Sweet(SweetTheme),
}

impl SystemTheme {
    /// Creates a new `SystemTheme` based on the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Theme configuration to use for theme selection
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    /// use nptk_theme::theme::system::SystemTheme;
    ///
    /// let config = ThemeConfig::from_env_or_default();
    /// let theme = SystemTheme::from_config(&config);
    /// ```
    pub fn from_config(config: &ThemeConfig) -> Self {
        match &config.default_theme {
            ThemeSource::Dark => SystemTheme::Dark(DarkTheme::new()),
            ThemeSource::Light => SystemTheme::Light(CelesteTheme::light()),
            ThemeSource::Sweet => SystemTheme::Sweet(SweetTheme::new()),
            _ => SystemTheme::Dark(DarkTheme::new()), // Default fallback
        }
    }
}

impl Default for SystemTheme {
    /// Creates a `SystemTheme` based on environment variables or default settings.
    ///
    /// This will read the `NPTK_THEME` environment variable if set,
    /// otherwise it defaults to dark theme.
    fn default() -> Self {
        let config = ThemeConfig::from_env_or_default();
        Self::from_config(&config)
    }
}

impl Theme for SystemTheme {
    fn get_property(&self, id: WidgetId, property: &ThemeProperty) -> Option<Color> {
        match self {
            SystemTheme::Light(theme) => theme.get_property(id, property),
            SystemTheme::Dark(theme) => theme.get_property(id, property),
            SystemTheme::Sweet(theme) => theme.get_property(id, property),
        }
    }

    fn style(&self, id: WidgetId) -> Option<crate::properties::ThemeStyle> {
        match self {
            SystemTheme::Light(theme) => theme.style(id),
            SystemTheme::Dark(theme) => theme.style(id),
            SystemTheme::Sweet(theme) => theme.style(id),
        }
    }

    fn window_background(&self) -> Color {
        match self {
            SystemTheme::Light(theme) => theme.window_background(),
            SystemTheme::Dark(theme) => theme.window_background(),
            SystemTheme::Sweet(theme) => theme.window_background(),
        }
    }

    fn globals(&self) -> &Globals {
        match self {
            SystemTheme::Light(theme) => theme.globals(),
            SystemTheme::Dark(theme) => theme.globals(),
            SystemTheme::Sweet(theme) => theme.globals(),
        }
    }

    fn globals_mut(&mut self) -> &mut Globals {
        match self {
            SystemTheme::Light(theme) => theme.globals_mut(),
            SystemTheme::Dark(theme) => theme.globals_mut(),
            SystemTheme::Sweet(theme) => theme.globals_mut(),
        }
    }

    fn widget_id(&self) -> WidgetId {
        match self {
            SystemTheme::Light(_) => WidgetId::new("nptk-theme", "SystemTheme-Light"),
            SystemTheme::Dark(_) => WidgetId::new("nptk-theme", "SystemTheme-Dark"),
            SystemTheme::Sweet(_) => WidgetId::new("nptk-theme", "SystemTheme-Sweet"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
