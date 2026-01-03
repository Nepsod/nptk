//! # Application Integration
//!
//! This module provides convenience traits and functions for integrating
//! the theme configuration system with NPTK applications.
//!
//! ## Overview
//!
//! The application integration module provides:
//!
//! - **[ThemeConfigurable]**: Trait for applications that support theme configuration
//! - **[DefaultThemeProvider]**: Trait for providing default themes
//! - **Convenience functions**: Easy integration with NPTK applications
//!
//! ## Key Features
//!
//! - **Easy Integration**: Simple trait implementation for theme configuration
//! - **Environment Variable Support**: Automatic theme loading from environment variables
//! - **Fallback Support**: Graceful fallback to default themes
//! - **Runtime Switching**: Support for runtime theme switching
//! - **Configuration Validation**: Validate theme configurations before use
//!
//! ## Usage Examples
//!
//! ### Basic Application Integration
//!
//! ```rust
//! use nptk_theme::app_integration::ThemeConfigurable;
//! use nptk_theme::config::ThemeConfig;
//! use nptk_theme::theme::Theme;
//!
//! struct MyApp;
//!
//! impl ThemeConfigurable for MyApp {
//!     fn theme_config(&self) -> ThemeConfig {
//!         ThemeConfig::from_env_or_default()
//!     }
//! }
//! ```
//!
//! ### Custom Theme Provider
//!
//! ```rust
//! use nptk_theme::app_integration::DefaultThemeProvider;
//! use nptk_theme::theme::dark::DarkTheme;
//!
//! struct MyApp;
//!
//! impl DefaultThemeProvider for MyApp {
//!     fn default_theme() -> Box<dyn Theme + Send + Sync> {
//!         Box::new(DarkTheme::new())
//!     }
//! }
//! ```
//!
//! ### Complete Application Example
//!
//! ```rust
//! use nptk_theme::app_integration::{ThemeConfigurable, DefaultThemeProvider};
//! use nptk_theme::config::ThemeConfig;
//! use nptk_theme::theme::{Theme, dark::DarkTheme};
//! use nptk::core::app::Application;
//! use nptk::core::config::MayConfig;
//!
//! struct MyApp;
//!
//! impl Application for MyApp {
//!     type Theme = Box<dyn Theme + Send + Sync>;
//!     type State = ();
//!
//!     fn config(&self) -> MayConfig<Self::Theme> {
//!         let theme_config = self.theme_config();
//!         let theme = theme_config.resolve_theme()
//!             .unwrap_or_else(|_| Self::default_theme());
//!         
//!         MayConfig {
//!             theme,
//!             ..Default::default()
//!         }
//!     }
//! }
//!
//! impl ThemeConfigurable for MyApp {
//!     fn theme_config(&self) -> ThemeConfig {
//!         ThemeConfig::from_env_or_default()
//!     }
//! }
//!
//! impl DefaultThemeProvider for MyApp {
//!     fn default_theme() -> Box<dyn Theme + Send + Sync> {
//!         Box::new(DarkTheme::new())
//!     }
//! }
//! ```
//!
//! ## Best Practices
//!
//! 1. **Use Environment Variables**: Set themes via environment variables for deployment
//! 2. **Provide Fallbacks**: Always provide fallback themes for robustness
//! 3. **Validate Configurations**: Validate theme configurations before use
//! 4. **Document Custom Themes**: Document any custom themes you create
//! 5. **Test Theme Switching**: Test theme switching functionality thoroughly
//!
//! ## Performance Considerations
//!
//! - **Lazy Loading**: Themes are loaded only when needed
//! - **Caching**: Theme configurations are cached for performance
//! - **Validation**: Theme validation is performed once at startup
//! - **Memory Usage**: Minimal memory overhead for configuration

use std::sync::Arc;

use crate::config::ThemeConfig;
use crate::manager::ThemeManager;
use crate::theme::Theme;
use crate::theme_resolver::SelfContainedThemeResolver;

/// A trait for applications that support theme configuration.
///
/// This trait provides a standardized way for applications to configure
/// themes through environment variables, configuration files, or programmatic
/// configuration.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::app_integration::ThemeConfigurable;
/// use nptk_theme::config::ThemeConfig;
///
/// struct MyApp;
///
/// impl ThemeConfigurable for MyApp {
///     fn theme_config(&self) -> ThemeConfig {
///         ThemeConfig::from_env_or_default()
///     }
/// }
/// ```
pub trait ThemeConfigurable {
    /// Get the theme configuration for this application.
    ///
    /// This method should return a theme configuration that specifies
    /// how themes should be loaded and configured for the application.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::app_integration::ThemeConfigurable;
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// struct MyApp;
    ///
    /// impl ThemeConfigurable for MyApp {
    ///     fn theme_config(&self) -> ThemeConfig {
    ///         ThemeConfig::from_env_or_default()
    ///     }
    /// }
    /// ```
    fn theme_config(&self) -> ThemeConfig;

    /// Get the theme manager for runtime theme switching.
    ///
    /// This method provides access to the theme manager for applications
    /// that need to switch themes at runtime.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::app_integration::ThemeConfigurable;
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// struct MyApp;
    ///
    /// impl ThemeConfigurable for MyApp {
    ///     fn theme_config(&self) -> ThemeConfig {
    ///         ThemeConfig::from_env_or_default()
    ///     }
    /// }
    ///
    /// let app = MyApp;
    /// let manager = app.theme_manager();
    /// ```
    fn theme_manager(&self) -> Arc<std::sync::RwLock<ThemeManager>> {
        self.theme_config().get_theme_manager()
    }
}

/// A trait for applications that provide default themes.
///
/// This trait allows applications to specify a default theme that will be
/// used when theme configuration fails or when no theme is specified.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::app_integration::DefaultThemeProvider;
/// use nptk_theme::theme::{Theme, dark::DarkTheme};
///
/// struct MyApp;
///
/// impl DefaultThemeProvider for MyApp {
///     fn default_theme() -> Box<dyn Theme + Send + Sync> {
///         Box::new(DarkTheme::new())
///     }
/// }
/// ```
pub trait DefaultThemeProvider {
    /// Get the default theme for this application.
    ///
    /// This method should return a theme that will be used as a fallback
    /// when theme configuration fails or when no theme is specified.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::app_integration::DefaultThemeProvider;
    /// use nptk_theme::theme::{Theme, dark::DarkTheme};
    ///
    /// struct MyApp;
    ///
    /// impl DefaultThemeProvider for MyApp {
    ///     fn default_theme() -> Box<dyn Theme + Send + Sync> {
    ///         Box::new(DarkTheme::new())
    ///     }
    /// }
    /// ```
    fn default_theme() -> Box<dyn Theme + Send + Sync>;
}

/// A convenience trait that combines theme configuration and default theme provision.
///
/// This trait provides a complete solution for applications that need both
/// theme configuration and default theme provision.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::app_integration::{ThemeConfigurable, DefaultThemeProvider, ThemeAware};
/// use nptk_theme::config::ThemeConfig;
/// use nptk_theme::theme::{Theme, dark::DarkTheme};
///
/// struct MyApp;
///
/// impl ThemeConfigurable for MyApp {
///     fn theme_config(&self) -> ThemeConfig {
///         ThemeConfig::from_env_or_default()
///     }
/// }
///
/// impl DefaultThemeProvider for MyApp {
///     fn default_theme() -> Box<dyn Theme + Send + Sync> {
///         Box::new(DarkTheme::new())
///     }
/// }
///
/// impl ThemeAware for MyApp {}
/// ```
pub trait ThemeAware: ThemeConfigurable + DefaultThemeProvider {
    /// Resolve the theme for this application.
    ///
    /// This method attempts to resolve the theme from the configuration,
    /// falling back to the default theme if configuration fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::app_integration::{ThemeConfigurable, DefaultThemeProvider, ThemeAware};
    /// use nptk_theme::config::ThemeConfig;
    /// use nptk_theme::theme::{Theme, dark::DarkTheme};
    ///
    /// struct MyApp;
    ///
    /// impl ThemeConfigurable for MyApp {
    ///     fn theme_config(&self) -> ThemeConfig {
    ///         ThemeConfig::from_env_or_default()
    ///     }
    /// }
    ///
    /// impl DefaultThemeProvider for MyApp {
    ///     fn default_theme() -> Box<dyn Theme + Send + Sync> {
    ///         Box::new(DarkTheme::new())
    ///     }
    /// }
    ///
    /// impl ThemeAware for MyApp {}
    ///
    /// let app = MyApp;
    /// let theme = app.resolve_theme().unwrap();
    /// ```
    fn resolve_theme(&self) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        self.theme_config()
            .resolve_theme()
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))) as Box<dyn std::error::Error>)
            .or_else(|_| Ok(Self::default_theme()))
    }

    /// Get the theme manager with the resolved theme.
    ///
    /// This method returns a theme manager that has been configured with
    /// the resolved theme for this application.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::app_integration::{ThemeConfigurable, DefaultThemeProvider, ThemeAware};
    /// use nptk_theme::config::ThemeConfig;
    /// use nptk_theme::theme::{Theme, dark::DarkTheme};
    ///
    /// struct MyApp;
    ///
    /// impl ThemeConfigurable for MyApp {
    ///     fn theme_config(&self) -> ThemeConfig {
    ///         ThemeConfig::from_env_or_default()
    ///     }
    /// }
    ///
    /// impl DefaultThemeProvider for MyApp {
    ///     fn default_theme() -> Box<dyn Theme + Send + Sync> {
    ///         Box::new(DarkTheme::new())
    ///     }
    /// }
    ///
    /// impl ThemeAware for MyApp {}
    ///
    /// let app = MyApp;
    /// let manager = app.theme_manager_with_resolved_theme().unwrap();
    /// ```
    fn theme_manager_with_resolved_theme(
        &self,
    ) -> Result<Arc<std::sync::RwLock<ThemeManager>>, Box<dyn std::error::Error>> {
        let theme = self.resolve_theme()?;
        let manager = ThemeManager::with_theme(theme);
        Ok(Arc::new(std::sync::RwLock::new(manager)))
    }
}

/// A convenience function to create a theme configuration from environment variables.
///
/// This function is a shorthand for `ThemeConfig::from_env_or_default()`.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::app_integration::create_app_theme_config;
///
/// let config = create_app_theme_config();
/// ```
pub fn create_app_theme_config() -> ThemeConfig {
    ThemeConfig::from_env_or_default()
}

/// A convenience function to resolve a theme for an application.
///
/// This function creates a theme configuration from environment variables
/// and resolves the theme, falling back to a default theme if needed.
///
/// # Arguments
///
/// * `default_theme_name` - The name of the default theme to use if configuration fails
///
/// # Examples
///
/// ```rust
/// use nptk_theme::app_integration::resolve_app_theme;
///
/// let theme = resolve_app_theme("dark").unwrap();
/// ```
pub fn resolve_app_theme(
    default_theme_name: &str,
) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
    let config = ThemeConfig::from_env_or_default();
    let resolver = SelfContainedThemeResolver::new();

    match resolver.resolve_from_config(&config) {
        Ok(theme) => Ok(theme),
        Err(_) => resolver.resolve_theme(default_theme_name)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))) as Box<dyn std::error::Error>),
    }
}
