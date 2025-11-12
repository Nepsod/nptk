//! # Theme Manager
//!
//! This module provides theme management functionality for the NPTK theming system,
//! including runtime theme switching, caching, and thread-safe theme access.
//!
//! ## Overview
//!
//! The theme manager system consists of:
//!
//! - **[ThemeManager]**: Main theme manager with runtime switching and caching
//! - **[ThemeVariant]**: Enum representing different theme variants
//! - **[SharedThemeManager]**: Thread-safe theme manager for multi-threaded applications
//!
//! ## Key Features
//!
//! - **Runtime Theme Switching**: Switch themes without restarting the application
//! - **Theme Caching**: Cache frequently accessed properties for better performance
//! - **Thread Safety**: Thread-safe theme management for multi-threaded applications
//! - **Multiple Theme Support**: Support for multiple theme variants
//! - **Variable Caching**: Cache theme variables for faster access
//!
//! ## Usage Examples
//!
//! ### Basic Theme Management
//!
//! ```rust
//! use nptk_theme::manager::{ThemeManager, ThemeVariant};
//! use nptk_theme::properties::ThemeProperty;
//! use nptk_theme::id::WidgetId;
//! use peniko::Color;
//!
//! // Create a theme manager
//! let mut manager = ThemeManager::new();
//!
//! // Switch to dark theme
//! manager.switch_theme(&ThemeVariant::Dark);
//!
//! // Get theme properties with caching
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let color = manager.get_property(button_id, &ThemeProperty::ColorIdle);
//! ```
//!
//! ### Thread-Safe Theme Management
//!
//! ```rust
//! use nptk_theme::manager::{create_shared_theme_manager, ThemeVariant};
//! use std::sync::Arc;
//! use std::thread;
//!
//! // Create a shared theme manager
//! let shared_manager = create_shared_theme_manager();
//!
//! // Use in multiple threads
//! let manager1 = shared_manager.clone();
//! let manager2 = shared_manager.clone();
//!
//! let handle1 = thread::spawn(move || {
//!     if let Ok(mut manager) = manager1.write() {
//!         manager.switch_theme(&ThemeVariant::Dark);
//!     }
//! });
//!
//! let handle2 = thread::spawn(move || {
//!     if let Ok(manager) = manager2.read() {
//!         let variants = manager.available_variants();
//!         println!("Available variants: {:?}", variants);
//!     }
//! });
//!
//! handle1.join().unwrap();
//! handle2.join().unwrap();
//! ```
//!
//! ### Custom Theme Management
//!
//! ```rust
//! use nptk_theme::manager::{ThemeManager, ThemeVariant};
//! use nptk_theme::theme::dark::DarkTheme;
//! use std::sync::Arc;
//!
//! // Create a theme manager with a custom theme
//! let custom_theme = Box::new(DarkTheme::new());
//! let mut manager = ThemeManager::with_theme(custom_theme);
//!
//! // Add additional themes
//! manager.add_theme(ThemeVariant::Custom("MyTheme".to_string()), Box::new(DarkTheme::new()));
//!
//! // Switch to custom theme
//! manager.switch_theme(&ThemeVariant::Custom("MyTheme".to_string()));
//! ```
//!
//! ### Theme Variable Access
//!
//! ```rust
//! use nptk_theme::manager::ThemeManager;
//! use nptk_theme::properties::ThemeValue;
//! use peniko::Color;
//!
//! let manager = ThemeManager::new();
//!
//! // Get theme variables with caching
//! let primary_color = manager.get_variable_color("primary");
//! let border_radius = manager.get_variable("border_radius");
//!
//! // Use variables
//! if let Some(color) = primary_color {
//!     println!("Primary color: {:?}", color);
//! }
//! ```
//!
//! ## Performance Features
//!
//! ### Caching
//!
//! The theme manager caches frequently accessed properties and variables:
//!
//! ```rust
//! use nptk_theme::manager::ThemeManager;
//! use nptk_theme::properties::ThemeProperty;
//! use nptk_theme::id::WidgetId;
//!
//! let manager = ThemeManager::new();
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//!
//! // First access - loads from theme
//! let color1 = manager.get_property(button_id, &ThemeProperty::ColorIdle);
//!
//! // Second access - uses cache (faster)
//! let color2 = manager.get_property(button_id, &ThemeProperty::ColorIdle);
//!
//! // Clear cache when needed
//! manager.clear_caches();
//! ```
//!
//! ### Thread Safety
//!
//! The theme manager is thread-safe and can be used across multiple threads:
//!
//! ```rust
//! use nptk_theme::manager::create_shared_theme_manager;
//! use std::sync::Arc;
//! use std::thread;
//!
//! let shared_manager = create_shared_theme_manager();
//!
//! // Multiple threads can safely access the theme manager
//! for i in 0..4 {
//!     let manager = shared_manager.clone();
//!     thread::spawn(move || {
//!         if let Ok(manager) = manager.read() {
//!             // Safe to read from multiple threads
//!             let variants = manager.available_variants();
//!             println!("Thread {}: {:?}", i, variants);
//!         }
//!     });
//! }
//! ```
//!
//! ## Theme Variants
//!
//! The [ThemeVariant] enum represents different theme types:
//!
//! ```rust
//! use nptk_theme::manager::ThemeVariant;
//!
//! // Built-in variants
//! let light = ThemeVariant::Light;
//! let dark = ThemeVariant::Dark;
//!
//! // Custom variants
//! let custom = ThemeVariant::Custom("MyCustomTheme".to_string());
//! ```
//!
//! ## Best Practices
//!
//! 1. **Use Shared Manager**: Use [SharedThemeManager] for multi-threaded applications
//! 2. **Cache Management**: Clear caches when switching themes
//! 3. **Error Handling**: Always handle potential errors when accessing shared managers
//! 4. **Performance**: Use caching for frequently accessed properties
//! 5. **Thread Safety**: Use read locks for read-only access, write locks for modifications
//!
//! ## Performance Considerations
//!
//! - **Caching**: Properties and variables are cached for faster access
//! - **Thread Safety**: Uses RwLock for efficient concurrent access
//! - **Memory Usage**: Caches are cleared when switching themes
//! - **Lock Contention**: Minimize lock contention by using appropriate lock types

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeValue};
use crate::theme::{celeste::CelesteTheme, dark::DarkTheme, Theme};

/// A theme variant that can be switched at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThemeVariant {
    /// Light theme variant.
    Light,
    /// Dark theme variant.
    Dark,
    /// Custom theme variant with a name.
    Custom(String),
}

impl Default for ThemeVariant {
    fn default() -> Self {
        Self::Light
    }
}

/// A theme manager that supports runtime theme switching and caching.
pub struct ThemeManager {
    current_theme: Arc<RwLock<Box<dyn Theme + Send + Sync>>>,
    theme_cache: Arc<RwLock<HashMap<(WidgetId, ThemeProperty), ThemeValue>>>,
    variables_cache: Arc<RwLock<HashMap<String, ThemeValue>>>,
    available_themes: HashMap<ThemeVariant, Box<dyn Theme + Send + Sync>>,
}

impl ThemeManager {
    /// Create a new theme manager with the default light theme.
    pub fn new() -> Self {
        let mut manager = Self {
            current_theme: Arc::new(RwLock::new(Box::new(CelesteTheme::light()))),
            theme_cache: Arc::new(RwLock::new(HashMap::new())),
            variables_cache: Arc::new(RwLock::new(HashMap::new())),
            available_themes: HashMap::new(),
        };

        // Add default themes
        manager.add_theme(ThemeVariant::Light, Box::new(CelesteTheme::light()));
        manager.add_theme(ThemeVariant::Dark, Box::new(DarkTheme::new()));

        manager
    }

    /// Create a new theme manager with a specific theme.
    pub fn with_theme(theme: Box<dyn Theme + Send + Sync>) -> Self {
        let mut manager = Self {
            current_theme: Arc::new(RwLock::new(theme)),
            theme_cache: Arc::new(RwLock::new(HashMap::new())),
            variables_cache: Arc::new(RwLock::new(HashMap::new())),
            available_themes: HashMap::new(),
        };

        // Add default themes
        manager.add_theme(ThemeVariant::Light, Box::new(CelesteTheme::light()));
        manager.add_theme(ThemeVariant::Dark, Box::new(DarkTheme::new()));

        manager
    }

    /// Add a theme variant to the manager.
    pub fn add_theme(&mut self, variant: ThemeVariant, theme: Box<dyn Theme + Send + Sync>) {
        self.available_themes.insert(variant, theme);
    }

    /// Switch to a different theme variant.
    pub fn switch_theme(&mut self, variant: &ThemeVariant) -> bool {
        if let Some(theme) = self.available_themes.get(variant) {
            // Clone the theme (themes should be lightweight to clone)
            let new_theme = self.clone_theme(theme.as_ref());
            if let Ok(mut current) = self.current_theme.write() {
                *current = new_theme;
                // Clear caches when switching themes
                self.clear_caches();
                return true;
            }
        }
        false
    }

    /// Get the current theme variant.
    pub fn current_variant(&self) -> ThemeVariant {
        // This is a simplified implementation - in practice, you'd track the current variant
        ThemeVariant::Light // Default fallback
    }

    /// Get all available theme variants.
    pub fn available_variants(&self) -> Vec<ThemeVariant> {
        self.available_themes.keys().cloned().collect()
    }

    /// Get a theme property with caching.
    pub fn get_property(&self, id: WidgetId, property: &ThemeProperty) -> Option<peniko::Color> {
        let cache_key = (id.clone(), property.clone());

        // Check cache first
        if let Ok(cache) = self.theme_cache.read() {
            if let Some(ThemeValue::Color(color)) = cache.get(&cache_key) {
                return Some(*color);
            }
        }

        // Get from current theme
        if let Ok(theme) = self.current_theme.read() {
            if let Some(color) = theme.get_property(id.clone(), property) {
                // Cache the result
                if let Ok(mut cache) = self.theme_cache.write() {
                    cache.insert(cache_key, ThemeValue::Color(color));
                }
                return Some(color);
            }
        }

        None
    }

    /// Get a theme variable with caching.
    pub fn get_variable(&self, name: &str) -> Option<ThemeValue> {
        // Check cache first
        if let Ok(cache) = self.variables_cache.read() {
            if let Some(value) = cache.get(name) {
                return Some(value.clone());
            }
        }

        // Get from current theme
        if let Ok(theme) = self.current_theme.read() {
            if let Some(value) = theme.variables().get(name) {
                // Cache the result
                if let Ok(mut cache) = self.variables_cache.write() {
                    cache.insert(name.to_string(), value.clone());
                }
                return Some(value.clone());
            }
        }

        None
    }

    /// Get a theme variable as a color.
    pub fn get_variable_color(&self, name: &str) -> Option<peniko::Color> {
        self.get_variable(name).and_then(|value| value.as_color())
    }

    /// Clear all caches.
    pub fn clear_caches(&self) {
        if let Ok(mut cache) = self.theme_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.variables_cache.write() {
            cache.clear();
        }
    }

    /// Get the current theme (for advanced usage).
    pub fn current_theme(&self) -> Arc<RwLock<Box<dyn Theme + Send + Sync>>> {
        self.current_theme.clone()
    }

    /// Clone a theme (helper method).
    fn clone_theme(&self, theme: &dyn Theme) -> Box<dyn Theme + Send + Sync> {
        // Try to downcast to known theme types and clone them
        if let Some(celeste_theme) = theme.as_any().downcast_ref::<CelesteTheme>() {
            Box::new(celeste_theme.clone())
        } else if let Some(dark_theme) = theme.as_any().downcast_ref::<DarkTheme>() {
            Box::new(dark_theme.clone())
        } else {
            // Fallback: create a new Celeste theme
            // This is not ideal but ensures we always return a valid theme
            log::warn!("Unknown theme type, falling back to Celeste theme");
            Box::new(CelesteTheme::light())
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe theme manager that can be shared across threads.
pub type SharedThemeManager = Arc<RwLock<ThemeManager>>;

/// Create a new shared theme manager.
pub fn create_shared_theme_manager() -> SharedThemeManager {
    Arc::new(RwLock::new(ThemeManager::new()))
}

/// Create a shared theme manager with a specific theme.
pub fn create_shared_theme_manager_with_theme(
    theme: Box<dyn Theme + Send + Sync>,
) -> SharedThemeManager {
    Arc::new(RwLock::new(ThemeManager::with_theme(theme)))
}
