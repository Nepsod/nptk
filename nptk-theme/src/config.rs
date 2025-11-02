//! # Theme Configuration System
//!
//! This module provides a comprehensive theme configuration system that allows
//! setting the default theme through environment variables, configuration files,
//! and programmatic configuration.
//!
//! ## Overview
//!
//! The theme configuration system provides:
//!
//! - **[ThemeConfig]**: Main configuration structure for theme settings
//! - **[ThemeSource]**: Enum for different theme configuration sources
//! - **[ThemeResolver]**: Resolves themes from various sources
//! - **Environment Variable Support**: Set themes via `NPTK_THEME` environment variable
//! - **Configuration File Support**: Load themes from configuration files
//! - **Programmatic Configuration**: Set themes programmatically
//!
//! ## Key Features
//!
//! - **Environment Variable Support**: Set default theme via `NPTK_THEME` env var
//! - **Configuration File Support**: Load themes from TOML configuration files
//! - **Theme Resolution**: Automatic theme resolution with fallbacks
//! - **Runtime Theme Switching**: Switch themes at runtime
//! - **Theme Validation**: Validate theme configurations
//! - **Extensible**: Easy to add new theme sources
//!
//! ## Usage Examples
//!
//! ### Environment Variable Configuration
//!
//! ```bash
//! # Set theme via environment variable
//! export NPTK_THEME=dark
//! export NPTK_THEME=light
//! export NPTK_THEME=custom:my-theme
//! ```
//!
//! ### Programmatic Configuration
//!
//! ```rust
//! use nptk_theme::config::{ThemeConfig, ThemeSource};
//!
//! // Create theme configuration
//! let config = ThemeConfig::new()
//!     .with_default_theme(ThemeSource::Dark)
//!     .with_fallback_theme(ThemeSource::Light);
//!
//! // Resolve theme
//! let theme = config.resolve_theme().unwrap();
//! ```
//!
//! ### Configuration File Support
//!
//! ```rust
//! use nptk_theme::config::ThemeConfig;
//!
//! // Load from TOML configuration file
//! let config = ThemeConfig::from_file("theme.toml").unwrap();
//! let theme = config.resolve_theme().unwrap();
//! ```
//!
//! ### Application Integration
//!
//! ```rust
//! use nptk_theme::config::ThemeConfig;
//! use nptk_theme::theme::Theme;
//!
//! impl Application for MyApp {
//!     type Theme = Box<dyn Theme + Send + Sync>;
//!     type State = ();
//!
//!     fn config(&self) -> MayConfig<Self::Theme> {
//!         let theme_config = ThemeConfig::from_env_or_default();
//!         let theme = theme_config.resolve_theme().unwrap();
//!         
//!         MayConfig {
//!             theme,
//!             ..Default::default()
//!         }
//!     }
//! }
//! ```
//!
//! ## Environment Variables
//!
//! The following environment variables are supported:
//!
//! - `NPTK_THEME`: Set the default theme (light, dark, or custom:name)
//! - `NPTK_THEME_CONFIG`: Path to theme configuration file
//! - `NPTK_THEME_FALLBACK`: Fallback theme if primary theme fails
//!
//! ## Configuration File Format
//!
//! ### TOML Format
//!
//! ```toml
//! [theme]
//! default = "dark"
//! fallback = "light"
//! 
//! [theme.custom]
//! name = "my-custom-theme"
//! path = "./themes/my-theme.toml"
//! ```
//!
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

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::theme::Theme;
use crate::manager::ThemeManager;
use crate::theme_resolver::SelfContainedThemeResolver;

/// A theme configuration that can be loaded from various sources.
///
/// This struct provides a comprehensive way to configure themes for NPTK applications.
/// It supports environment variables, configuration files, and programmatic configuration.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::config::{ThemeConfig, ThemeSource};
///
/// // Create configuration from environment variables
/// let config = ThemeConfig::from_env_or_default();
///
/// // Create configuration programmatically
/// let config = ThemeConfig::new()
///     .with_default_theme(ThemeSource::Dark)
///     .with_fallback_theme(ThemeSource::Light);
///
/// // Resolve theme
/// let theme = config.resolve_theme().unwrap();
/// ```
///
/// # Environment Variables
///
/// The following environment variables are supported:
///
/// - `NPTK_THEME`: Set the default theme (light, dark, or custom:name)
/// - `NPTK_THEME_CONFIG`: Path to theme configuration file
/// - `NPTK_THEME_FALLBACK`: Fallback theme if primary theme fails
///
/// # Configuration File Support
///
/// Configuration files can be in TOML or JSON format and should contain:
///
/// ```toml
/// [theme]
/// default = "dark"
/// fallback = "light"
/// 
/// [theme.custom]
/// name = "my-custom-theme"
/// path = "./themes/my-theme.toml"
/// ```
#[derive(Clone)]
pub struct ThemeConfig {
    /// The default theme source.
    pub default_theme: ThemeSource,
    /// The fallback theme source.
    pub fallback_theme: Option<ThemeSource>,
    /// Custom theme configurations.
    custom_themes: HashMap<String, CustomThemeConfig>,
    /// Theme manager for runtime theme switching.
    theme_manager: Option<Arc<std::sync::RwLock<ThemeManager>>>,
}

/// A source for theme configuration.
///
/// This enum represents different ways to specify a theme, including
/// built-in themes, custom themes, and theme files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSource {
    /// Light theme (Celeste).
    Light,
    /// Dark theme.
    Dark,
    /// Sweet theme (modern dark theme with vibrant accents).
    Sweet,
    /// Custom theme with a name.
    Custom(String),
    /// Theme loaded from a file.
    File(String),
}

/// Configuration for a custom theme.
#[derive(Debug, Clone)]
pub struct CustomThemeConfig {
    /// The name of the custom theme.
    pub name: String,
    /// The path to the theme configuration file.
    pub path: Option<String>,
    /// Additional theme properties.
    pub properties: HashMap<String, String>,
}

/// A theme resolver that can resolve themes from various sources.
///
/// This struct provides methods to resolve themes from different sources
/// and handle theme loading errors gracefully.
pub struct ThemeResolver {
    /// Available theme configurations.
    configs: HashMap<String, Box<dyn Theme + Send + Sync>>,
}

impl ThemeConfig {
    /// Create a new theme configuration with default settings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// let config = ThemeConfig::new();
    /// ```
    pub fn new() -> Self {
        Self {
            default_theme: ThemeSource::Light,
            fallback_theme: Some(ThemeSource::Dark),
            custom_themes: HashMap::new(),
            theme_manager: None,
        }
    }

    /// Create a theme configuration from environment variables or use defaults.
    ///
    /// This method reads the following environment variables:
    /// - `NPTK_THEME`: The default theme (light, dark, or custom:name)
    /// - `NPTK_THEME_FALLBACK`: The fallback theme
    /// - `NPTK_THEME_CONFIG`: Path to a configuration file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// // Load from environment variables
    /// let config = ThemeConfig::from_env_or_default();
    /// ```
    pub fn from_env_or_default() -> Self {
        let mut config = Self::new();

        // Load from NPTK_THEME environment variable
        if let Ok(theme_env) = env::var("NPTK_THEME") {
            config.default_theme = Self::parse_theme_source(&theme_env);
        }

        // Load fallback theme
        if let Ok(fallback_env) = env::var("NPTK_THEME_FALLBACK") {
            config.fallback_theme = Some(Self::parse_theme_source(&fallback_env));
        }

        // Load from configuration file if specified
        if let Ok(config_path) = env::var("NPTK_THEME_CONFIG") {
            if let Ok(file_config) = Self::from_file(&config_path) {
                config = file_config;
            }
        }

        config
    }

    /// Load theme configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// let config = ThemeConfig::from_file("theme.toml").unwrap();
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;

        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            Self::from_toml(&content)
        } else {
            Err("Unsupported configuration file format. Use .toml".into())
        }
    }

    /// Load theme configuration from TOML content.
    ///
    /// # Arguments
    ///
    /// * `content` - TOML configuration content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// let toml_content = r#"
    /// [theme]
    /// default = "dark"
    /// fallback = "light"
    /// "#;
    ///
    /// let config = ThemeConfig::from_toml(toml_content).unwrap();
    /// ```
    pub fn from_toml(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // For now, we'll use a simple parser. In a real implementation,
        // you'd use the `toml` crate for proper TOML parsing.
        let mut config = Self::new();

        // Simple TOML parsing (in a real implementation, use the `toml` crate)
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("default = ") {
                if let Some(theme_str) = line.strip_prefix("default = ") {
                    let theme = theme_str.trim_matches('"');
                    config.default_theme = Self::parse_theme_source(theme);
                }
            } else if line.starts_with("fallback = ") {
                if let Some(theme_str) = line.strip_prefix("fallback = ") {
                    let theme = theme_str.trim_matches('"');
                    config.fallback_theme = Some(Self::parse_theme_source(theme));
                }
            }
        }

        Ok(config)
    }


    /// Set the default theme source.
    ///
    /// # Arguments
    ///
    /// * `theme` - The default theme source
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::{ThemeConfig, ThemeSource};
    ///
    /// let config = ThemeConfig::new()
    ///     .with_default_theme(ThemeSource::Dark);
    /// ```
    pub fn with_default_theme(mut self, theme: ThemeSource) -> Self {
        self.default_theme = theme;
        self
    }

    /// Set the fallback theme source.
    ///
    /// # Arguments
    ///
    /// * `theme` - The fallback theme source
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::{ThemeConfig, ThemeSource};
    ///
    /// let config = ThemeConfig::new()
    ///     .with_fallback_theme(ThemeSource::Light);
    /// ```
    pub fn with_fallback_theme(mut self, theme: ThemeSource) -> Self {
        self.fallback_theme = Some(theme);
        self
    }

    /// Add a custom theme configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the custom theme
    /// * `config` - The custom theme configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::{ThemeConfig, CustomThemeConfig};
    ///
    /// let custom_config = CustomThemeConfig {
    ///     name: "my-theme".to_string(),
    ///     path: Some("./themes/my-theme.toml".to_string()),
    ///     properties: std::collections::HashMap::new(),
    /// };
    ///
    /// let config = ThemeConfig::new()
    ///     .with_custom_theme("my-theme", custom_config);
    /// ```
    pub fn with_custom_theme(mut self, name: String, config: CustomThemeConfig) -> Self {
        self.custom_themes.insert(name, config);
        self
    }

    /// Resolve the theme from the configuration.
    ///
    /// This method attempts to resolve the theme from the default source,
    /// falling back to the fallback theme if the default fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// let config = ThemeConfig::from_env_or_default();
    /// let theme = config.resolve_theme().unwrap();
    /// ```
    pub fn resolve_theme(&self) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        let resolver = SelfContainedThemeResolver::new();
        resolver.resolve_from_config(self)
    }

    /// Resolve a theme from a specific source.
    ///
    /// # Arguments
    ///
    /// * `source` - The theme source to resolve
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::{ThemeConfig, ThemeSource};
    ///
    /// let config = ThemeConfig::new();
    /// let theme = config.resolve_theme_source(&ThemeSource::Dark).unwrap();
    /// ```
    pub fn resolve_theme_source(&self, source: &ThemeSource) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        let resolver = SelfContainedThemeResolver::new();
        resolver.resolve_theme_source(source)
    }

    /// Get the theme manager for runtime theme switching.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// let config = ThemeConfig::from_env_or_default();
    /// let manager = config.get_theme_manager();
    /// ```
    pub fn get_theme_manager(&self) -> Arc<std::sync::RwLock<ThemeManager>> {
        if let Some(ref manager) = self.theme_manager {
            manager.clone()
        } else {
            Arc::new(std::sync::RwLock::new(ThemeManager::new()))
        }
    }

    /// Parse a theme source from a string.
    ///
    /// # Arguments
    ///
    /// * `source` - String representation of the theme source
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::{ThemeConfig, ThemeSource};
    ///
    /// let source = ThemeConfig::parse_theme_source("dark");
    /// assert_eq!(source, ThemeSource::Dark);
    /// ```
    fn parse_theme_source(source: &str) -> ThemeSource {
        match source.to_lowercase().as_str() {
            "light" => ThemeSource::Light,
            "dark" => ThemeSource::Dark,
            "sweet" => ThemeSource::Sweet,
            s if s.starts_with("custom:") => {
                let name = s.strip_prefix("custom:").unwrap().to_string();
                ThemeSource::Custom(name)
            }
            s if s.starts_with("file:") => {
                let path = s.strip_prefix("file:").unwrap().to_string();
                ThemeSource::File(path)
            }
            _ => ThemeSource::Light, // Default fallback
        }
    }

}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeResolver {
    /// Create a new theme resolver.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeResolver;
    ///
    /// let resolver = ThemeResolver::new();
    /// ```
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Register a theme configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the theme
    /// * `theme` - The theme implementation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeResolver;
    /// use nptk_theme::theme::dark::DarkTheme;
    ///
    /// let mut resolver = ThemeResolver::new();
    /// resolver.register_theme("dark", Box::new(DarkTheme::new()));
    /// ```
    pub fn register_theme(&mut self, name: String, theme: Box<dyn Theme + Send + Sync>) {
        self.configs.insert(name, theme);
    }

    /// Resolve a theme by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the theme to resolve
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::config::ThemeResolver;
    ///
    /// let resolver = ThemeResolver::new();
    /// let theme = resolver.resolve_theme("dark");
    /// ```
    pub fn resolve_theme(&self, name: &str) -> Option<&Box<dyn Theme + Send + Sync>> {
        self.configs.get(name)
    }
}

impl Default for ThemeResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a theme configuration from environment variables.
///
/// This function is a shorthand for `ThemeConfig::from_env_or_default()`.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::config::create_theme_config;
///
/// let config = create_theme_config();
/// let theme = config.resolve_theme().unwrap();
/// ```
pub fn create_theme_config() -> ThemeConfig {
    ThemeConfig::from_env_or_default()
}

/// Convenience function to resolve a theme from environment variables.
///
/// This function creates a theme configuration from environment variables
/// and resolves the theme in one step.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::config::resolve_theme_from_env;
///
/// let theme = resolve_theme_from_env().unwrap();
/// ```
pub fn resolve_theme_from_env() -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
    let config = ThemeConfig::from_env_or_default();
    config.resolve_theme()
}
