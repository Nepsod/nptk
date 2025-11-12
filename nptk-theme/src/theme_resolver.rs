//! # Theme Resolver
//!
//! This module provides a self-contained theme resolver that can create themes
//! without requiring applications to import specific theme implementations.
//!
//! ## Overview
//!
//! The theme resolver provides:
//!
//! - **[SelfContainedThemeResolver]**: Resolves themes without external imports
//! - **Built-in Theme Factory**: Creates built-in themes internally
//! - **Theme Registry**: Registry of available themes
//! - **Automatic Theme Loading**: Loads themes based on configuration
//!
//! ## Key Features
//!
//! - **Self-Contained**: No need to import specific theme implementations
//! - **Built-in Themes**: Automatically includes all built-in themes
//! - **Theme Registry**: Centralized theme management
//! - **Configuration-Driven**: Loads themes based on configuration
//! - **Extensible**: Easy to add new themes to the registry
//!
//! ## Usage Examples
//!
//! ### Basic Theme Resolution
//!
//! ```rust
//! use nptk_theme::theme_resolver::SelfContainedThemeResolver;
//!
//! let resolver = SelfContainedThemeResolver::new();
//! let theme = resolver.resolve_theme("dark").unwrap();
//! ```
//!
//! ### Application Integration
//!
//! ```rust
//! use nptk_theme::theme_resolver::SelfContainedThemeResolver;
//! use nptk_theme::config::ThemeConfig;
//!
//! let resolver = SelfContainedThemeResolver::new();
//! let config = ThemeConfig::from_env_or_default();
//! let theme = resolver.resolve_from_config(&config).unwrap();
//! ```
//!
//! ## Available Themes
//!
//! The resolver automatically includes these built-in themes:
//!
//! - `light` - Celeste light theme
//! - `dark` - Dark theme
//! - `celeste` - Celeste light theme (alias for light)
//!
//! ## Best Practices
//!
//! 1. **Use the Resolver**: Always use the resolver for theme creation
//! 2. **Check Availability**: Check if themes are available before using
//! 3. **Handle Errors**: Always handle theme resolution errors gracefully
//! 4. **Use Configuration**: Use configuration-driven theme resolution
//! 5. **Document Custom Themes**: Document any custom themes you add

use std::collections::HashMap;

use crate::config::{ThemeConfig, ThemeSource};
use crate::theme::{celeste::CelesteTheme, dark::DarkTheme, sweet::SweetTheme, Theme};

/// A self-contained theme resolver that can create themes without external imports.
///
/// This resolver automatically includes all built-in themes and provides
/// a centralized way to resolve themes based on configuration.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
///
/// let resolver = SelfContainedThemeResolver::new();
/// let theme = resolver.resolve_theme("dark").unwrap();
/// ```
///
/// # Available Themes
///
/// The resolver automatically includes these built-in themes:
///
/// - `light` - Celeste light theme
/// - `dark` - Dark theme
/// - `celeste` - Celeste light theme (alias for light)
pub struct SelfContainedThemeResolver {
    /// Registry of available themes.
    theme_registry: HashMap<String, Box<dyn Theme + Send + Sync>>,
}

impl SelfContainedThemeResolver {
    /// Create a new self-contained theme resolver.
    ///
    /// This method automatically registers all built-in themes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// ```
    pub fn new() -> Self {
        let mut resolver = Self {
            theme_registry: HashMap::new(),
        };

        // Register built-in themes
        resolver.register_builtin_themes();

        resolver
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
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// let theme = resolver.resolve_theme("dark").unwrap();
    /// ```
    pub fn resolve_theme(
        &self,
        name: &str,
    ) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        match name.to_lowercase().as_str() {
            "light" | "celeste" => Ok(Box::new(CelesteTheme::light())),
            "dark" => Ok(Box::new(DarkTheme::new())),
            _ => {
                // Check if it's a custom theme
                if let Some(_theme) = self.theme_registry.get(name) {
                    // Clone the theme (in a real implementation, you'd need proper cloning)
                    // For now, we'll create a new instance based on the name
                    self.create_theme_by_name(name)
                } else {
                    Err(format!("Theme '{}' not found", name).into())
                }
            },
        }
    }

    /// Resolve a theme from a theme source.
    ///
    /// # Arguments
    ///
    /// * `source` - The theme source to resolve
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    /// use nptk_theme::config::ThemeSource;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// let theme = resolver.resolve_theme_source(&ThemeSource::Dark).unwrap();
    /// ```
    pub fn resolve_theme_source(
        &self,
        source: &ThemeSource,
    ) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        match source {
            ThemeSource::Light => Ok(Box::new(CelesteTheme::light())),
            ThemeSource::Dark => Ok(Box::new(DarkTheme::new())),
            ThemeSource::Custom(name) => self.resolve_theme(name),
            ThemeSource::File(path) => {
                // Load theme from file
                let config = ThemeConfig::from_file(path)?;
                self.resolve_from_config(&config)
            },
            ThemeSource::Sweet => Ok(Box::new(SweetTheme::new())),
        }
    }

    /// Resolve a theme from a theme configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The theme configuration to resolve
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    /// use nptk_theme::config::ThemeConfig;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// let config = ThemeConfig::from_env_or_default();
    /// let theme = resolver.resolve_from_config(&config).unwrap();
    /// ```
    pub fn resolve_from_config(
        &self,
        config: &ThemeConfig,
    ) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        // Try to resolve the default theme
        match self.resolve_theme_source(&config.default_theme) {
            Ok(theme) => Ok(theme),
            Err(_) => {
                // Try fallback theme
                if let Some(ref fallback) = config.fallback_theme {
                    match self.resolve_theme_source(fallback) {
                        Ok(theme) => Ok(theme),
                        Err(e) => Err(format!("Failed to resolve fallback theme: {}", e).into()),
                    }
                } else {
                    Err("No fallback theme configured".into())
                }
            },
        }
    }

    /// Check if a theme is available.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the theme to check
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// let is_available = resolver.is_theme_available("dark");
    /// ```
    pub fn is_theme_available(&self, name: &str) -> bool {
        match name.to_lowercase().as_str() {
            "light" | "celeste" | "dark" => true,
            _ => self.theme_registry.contains_key(name),
        }
    }

    /// Get a list of available themes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// let themes = resolver.available_themes();
    /// ```
    pub fn available_themes(&self) -> Vec<String> {
        let mut themes = vec![
            "light".to_string(),
            "celeste".to_string(),
            "dark".to_string(),
        ];
        themes.extend(self.theme_registry.keys().cloned());
        themes.sort();
        themes
    }

    /// Register a custom theme.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the theme
    /// * `theme` - The theme implementation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    /// use nptk_theme::theme::dark::DarkTheme;
    ///
    /// let mut resolver = SelfContainedThemeResolver::new();
    /// resolver.register_theme("my-dark", Box::new(DarkTheme::new()));
    /// ```
    pub fn register_theme(&mut self, name: String, theme: Box<dyn Theme + Send + Sync>) {
        self.theme_registry.insert(name, theme);
    }

    /// Register all built-in themes.
    ///
    /// This method is called automatically during initialization.
    fn register_builtin_themes(&mut self) {
        // Built-in themes are resolved directly by name, so we don't need to register them
        // This method is here for future extensibility
        let _ = self; // Suppress unused variable warning
    }

    /// Create a theme by name (for custom themes).
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the theme to create
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::theme_resolver::SelfContainedThemeResolver;
    ///
    /// let resolver = SelfContainedThemeResolver::new();
    /// let theme = resolver.create_theme_by_name("dark").unwrap();
    /// ```
    fn create_theme_by_name(
        &self,
        name: &str,
    ) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
        match name.to_lowercase().as_str() {
            "light" | "celeste" => Ok(Box::new(CelesteTheme::light())),
            "dark" => Ok(Box::new(DarkTheme::new())),
            _ => Err(format!("Theme '{}' not found", name).into()),
        }
    }
}

impl Default for SelfContainedThemeResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new theme resolver instance.
///
/// This provides a convenient way to create theme resolvers.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::theme_resolver::create_theme_resolver;
///
/// let resolver = create_theme_resolver();
/// let theme = resolver.resolve_theme("dark").unwrap();
/// ```
pub fn create_theme_resolver() -> SelfContainedThemeResolver {
    SelfContainedThemeResolver::new()
}

/// Convenience function to resolve a theme by name.
///
/// This function creates a new theme resolver to resolve themes.
///
/// # Arguments
///
/// * `name` - The name of the theme to resolve
///
/// # Examples
///
/// ```rust
/// use nptk_theme::theme_resolver::resolve_theme;
///
/// let theme = resolve_theme("dark").unwrap();
/// ```
pub fn resolve_theme(
    name: &str,
) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
    let resolver = create_theme_resolver();
    resolver.resolve_theme(name)
}

/// Convenience function to resolve a theme from configuration.
///
/// This function creates a new theme resolver to resolve themes from configuration.
///
/// # Arguments
///
/// * `config` - The theme configuration to resolve
///
/// # Examples
///
/// ```rust
/// use nptk_theme::theme_resolver::resolve_theme_from_config;
/// use nptk_theme::config::ThemeConfig;
///
/// let config = ThemeConfig::from_env_or_default();
/// let theme = resolve_theme_from_config(&config).unwrap();
/// ```
pub fn resolve_theme_from_config(
    config: &ThemeConfig,
) -> Result<Box<dyn Theme + Send + Sync>, Box<dyn std::error::Error>> {
    let resolver = create_theme_resolver();
    resolver.resolve_from_config(config)
}
