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
//! use vello::peniko::Color;
//!
//! // Create a theme manager
//! let mut manager = ThemeManager::new();
//!
//! // Switch to dark theme
//! manager.switch_theme("dark");
//!
//! // Get theme properties with caching
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let color = manager.get_property(button_id, &ThemeProperty::ColorIdle);
//! ```
//!
//! ### Thread-Safe Theme Management
//!
//! ```rust
//! use nptk_theme::manager::{create_shared_theme_manager};
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
//!         manager.switch_theme("dark");
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
//! use nptk_theme::manager::{ThemeManager};
//! use nptk_theme::theme::dark::DarkTheme;
//! use std::sync::Arc;
//!
//! // Create a theme manager with a custom theme
//! let custom_theme = Box::new(DarkTheme::new());
//! let mut manager = ThemeManager::with_theme(custom_theme);
//!
//! // Add additional themes
//! manager.add_theme("MyTheme", Box::new(DarkTheme::new()));
//!
//! // Switch to custom theme
//! manager.switch_theme("MyTheme");
//! ```
//!
//! ### Theme Variable Access
//!
//! ```rust
//! use nptk_theme::manager::ThemeManager;
//! use nptk_theme::properties::ThemeValue;
//! use vello::peniko::Color;
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
//! Themes are identified by string names:
//!
//! ```rust
//! use nptk_theme::manager::ThemeManager;
//!
//! let mut manager = ThemeManager::new();
//!
//! // Built-in variants
//! manager.switch_theme("light");
//! manager.switch_theme("dark");
//!
//! // Custom variants
//! manager.add_theme("MyCustomTheme", Box::new(crate::theme::dark::DarkTheme::new()));
//! manager.switch_theme("MyCustomTheme");
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
use std::sync::{Arc, RwLock, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeValue};
use crate::theme::{celeste::CelesteTheme, dark::DarkTheme, sweet::SweetTheme, Theme};


// ThemeVariant enum is removed in favor of simple strings


/// A theme manager that supports runtime theme switching and caching.
pub struct ThemeManager {
    current_theme: Arc<RwLock<Box<dyn Theme + Send + Sync>>>,
    current_variant_internal: Arc<RwLock<String>>,
    theme_cache: Arc<RwLock<HashMap<(WidgetId, ThemeProperty), ThemeValue>>>,
    variables_cache: Arc<RwLock<HashMap<String, ThemeValue>>>,
    available_themes: HashMap<String, Box<dyn Theme + Send + Sync>>,
    change_notifiers: Arc<RwLock<Vec<mpsc::Sender<String>>>>,
    notify_counter: AtomicUsize,
}

impl ThemeManager {
    /// Create a new theme manager with the default Sweet theme.
    pub fn new() -> Self {
        let mut manager = Self {
            current_theme: Arc::new(RwLock::new(Box::new(SweetTheme::new()))),
            current_variant_internal: Arc::new(RwLock::new("sweet".to_string())),
            theme_cache: Arc::new(RwLock::new(HashMap::new())),
            variables_cache: Arc::new(RwLock::new(HashMap::new())),
            available_themes: HashMap::new(),
            change_notifiers: Arc::new(RwLock::new(Vec::new())),
            notify_counter: AtomicUsize::new(0),
        };

        // Add default themes
        manager.add_theme("light", Box::new(CelesteTheme::light()));
        manager.add_theme("dark", Box::new(DarkTheme::new()));
        manager.add_theme("sweet", Box::new(SweetTheme::new()));

        manager
    }

    /// Create a new theme manager with a specific theme.
    pub fn with_theme(theme: Box<dyn Theme + Send + Sync>) -> Self {
        let mut manager = Self {
            current_theme: Arc::new(RwLock::new(theme)),
            current_variant_internal: Arc::new(RwLock::new("custom".to_string())),
            theme_cache: Arc::new(RwLock::new(HashMap::new())),
            variables_cache: Arc::new(RwLock::new(HashMap::new())),
            available_themes: HashMap::new(),
            change_notifiers: Arc::new(RwLock::new(Vec::new())),
            notify_counter: AtomicUsize::new(0),
        };

        // Add default themes
        manager.add_theme("light", Box::new(CelesteTheme::light()));
        manager.add_theme("dark", Box::new(DarkTheme::new()));
        manager.add_theme("sweet", Box::new(SweetTheme::new()));

        manager
    }

    /// Add a theme variant to the manager.
    pub fn add_theme<S: Into<String>>(&mut self, name: S, theme: Box<dyn Theme + Send + Sync>) {
        self.available_themes.insert(name.into(), theme);
    }

    /// Switch to a different theme variant.
    pub fn switch_theme(&mut self, name: &str) -> bool {
        if let Some(theme) = self.available_themes.get(name) {
            // Clone the theme (themes should be lightweight to clone)
            let new_theme = self.clone_theme(theme.as_ref());
            if let Ok(mut current) = self.current_theme.write() {
                *current = new_theme;
                // Clear caches when switching themes
                self.clear_caches();
                
                // Update current variant
                if let Ok(mut current_var) = self.current_variant_internal.write() {
                    *current_var = name.to_string();
                }
                
                // Notify all subscribers of theme change
                self.notify_theme_changed(name.to_string());
                return true;
            }
        }
        false
    }

    /// Notify all subscribers that the theme has changed.
    fn notify_theme_changed(&self, name: String) {
        let mut notifiers = match self.change_notifiers.write() {
            Ok(n) => n,
            Err(_) => return,
        };
        
        // Remove dead receivers and send notifications
        notifiers.retain(|sender| {
            sender.send(name.clone()).is_ok()
        });
        
        self.notify_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Subscribe to theme change notifications.
    /// Returns a receiver that will receive notifications when the theme changes.
    pub fn subscribe_theme_changes(&self) -> mpsc::Receiver<String> {
        let (sender, receiver) = mpsc::channel();
        
        if let Ok(mut notifiers) = self.change_notifiers.write() {
            notifiers.push(sender);
        }
        
        receiver
    }

    /// Get the current theme variant name.
    pub fn current_variant(&self) -> String {
        self.current_variant_internal.read()
            .map(|v| v.clone())
            .unwrap_or_else(|_| "sweet".to_string())
    }

    /// Get all available theme variants.
    pub fn available_variants(&self) -> Vec<String> {
        self.available_themes.keys().cloned().collect()
    }

    /// Get a theme property with caching.
    pub fn get_property(&self, id: WidgetId, property: &ThemeProperty) -> Option<vello::peniko::Color> {
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
    pub fn get_variable_color(&self, name: &str) -> Option<vello::peniko::Color> {
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

    /// Access the current theme through a closure for rendering.
    /// This allows widgets to access theme properties without cloning.
    pub fn access_theme<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&dyn Theme) -> R,
    {
        self.current_theme.read().ok().map(|theme| {
            f(theme.as_ref())
        })
    }

    /// Access the current theme mutably through a closure.
    pub fn access_theme_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut dyn Theme) -> R,
    {
        self.current_theme.write().ok().map(|mut theme| {
            f(theme.as_mut())
        })
    }

    /// Clone a theme (helper method).
    fn clone_theme(&self, theme: &dyn Theme) -> Box<dyn Theme + Send + Sync> {
        // Try to downcast to known theme types and clone them
        if let Some(celeste_theme) = theme.as_any().downcast_ref::<CelesteTheme>() {
            Box::new(celeste_theme.clone())
        } else if let Some(dark_theme) = theme.as_any().downcast_ref::<DarkTheme>() {
            Box::new(dark_theme.clone())
        } else if let Some(sweet_theme) = theme.as_any().downcast_ref::<SweetTheme>() {
            Box::new(sweet_theme.clone())
        } else {
            // Fallback: create a new Sweet theme
            // This is not ideal but ensures we always return a valid theme
            log::warn!("Unknown theme type, falling back to Sweet theme");
            Box::new(SweetTheme::new())
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

/// Create a shared theme manager from a theme configuration.
///
/// This function resolves the theme from the configuration and initializes
/// the theme manager with it. If resolution fails, it falls back to the default theme.
pub fn create_shared_theme_manager_from_config(
    config: &crate::config::ThemeConfig,
) -> SharedThemeManager {
    use crate::config::ThemeSource;
    use crate::theme_resolver::SelfContainedThemeResolver;
    
    let resolver = SelfContainedThemeResolver::new();
    let mut manager = ThemeManager::new();
    
    // Try to resolve the theme from config
    if let Some(ref default_source) = config.default_theme {
        if let Ok(theme) = resolver.resolve_theme_source(default_source) {
            // Determine the variant based on the source
            let variant = match default_source {
                ThemeSource::Light => "light",
                ThemeSource::Dark => "dark",
                ThemeSource::Sweet => "sweet",
                ThemeSource::Custom(name) => name.as_str(),
                ThemeSource::File(_) => "custom", // File themes default to custom
            };
            
            // Add the theme and switch to it
            manager.add_theme(variant, theme);
            manager.switch_theme(variant);
        }
    } else {
        // No default theme specified, use Sweet as default
        manager.switch_theme("sweet");
    }
    
    Arc::new(RwLock::new(manager))
}

/// Create a shared theme manager with a specific theme.
pub fn create_shared_theme_manager_with_theme(
    theme: Box<dyn Theme + Send + Sync>,
) -> SharedThemeManager {
    Arc::new(RwLock::new(ThemeManager::with_theme(theme)))
}
