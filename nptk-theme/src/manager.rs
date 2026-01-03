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
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use notify::{Watcher, RecommendedWatcher, Event, EventKind, RecursiveMode};

use crate::error::ThemeError;
use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeValue};
use crate::theme::{celeste::CelesteTheme, dark::DarkTheme, sweet::SweetTheme, Theme};
use crate::transition::{ThemeTransition, TransitionConfig};


// ThemeVariant enum is removed in favor of simple strings


/// A theme manager that supports runtime theme switching and caching.
pub struct ThemeManager {
    current_theme: Arc<RwLock<Box<dyn Theme + Send + Sync>>>,
    current_variant_internal: Arc<RwLock<String>>,
    theme_cache: Arc<RwLock<HashMap<u64, ThemeValue>>>,
    variables_cache: Arc<RwLock<HashMap<String, ThemeValue>>>,
    available_themes: HashMap<String, Box<dyn Theme + Send + Sync>>,
    change_notifiers: Arc<RwLock<Vec<mpsc::Sender<String>>>>,
    notify_counter: AtomicUsize,
    active_transition: Option<Arc<RwLock<ThemeTransition>>>,
    file_watcher: Option<RecommendedWatcher>,
    watched_paths: Vec<PathBuf>,
    config_reload_sender: Option<mpsc::Sender<PathBuf>>,
    config_reload_receiver: Option<mpsc::Receiver<PathBuf>>,
    config_path: Option<PathBuf>,
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
            active_transition: None,
            file_watcher: None,
            watched_paths: Vec::new(),
            config_reload_sender: None,
            config_reload_receiver: None,
            config_path: None,
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
            active_transition: None,
            file_watcher: None,
            watched_paths: Vec::new(),
            config_reload_sender: None,
            config_reload_receiver: None,
            config_path: None,
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
        self.switch_theme_with_transition(name, None).is_ok()
    }

    /// Switch to a different theme variant with optional transition.
    ///
    /// If `transition_config` is `Some` and transitions are enabled, a smooth
    /// transition will be performed. Otherwise, the theme switches immediately.
    pub fn switch_theme_with_transition(
        &mut self,
        name: &str,
        transition_config: Option<&TransitionConfig>,
    ) -> Result<(), ThemeError> {
        let theme = self.available_themes.get(name)
            .ok_or_else(|| ThemeError::not_found(name))?;

        // Clone the current theme for transition start point
        let start_theme = if let Ok(current) = self.current_theme.read() {
            self.clone_theme(current.as_ref())
        } else {
            return Err(ThemeError::transition_error("Failed to access current theme"));
        };

        let target_theme = self.clone_theme(theme.as_ref());

        // Check if we should use transitions
        if let Some(config) = transition_config {
            if config.is_enabled() {
                // Clone target theme properly
                let target_theme_clone = self.clone_theme(target_theme.as_ref());
                let transition = ThemeTransition::new(
                    start_theme,
                    target_theme_clone,
                    config.duration(),
                );
                // Set transition first
                self.active_transition = Some(Arc::new(RwLock::new(transition)));
            } else {
                // Transition disabled, switch immediately
                if let Ok(mut current) = self.current_theme.write() {
                    *current = target_theme;
                }
                self.active_transition = None;
            }
        } else {
            // No transition config provided, switch immediately
            if let Ok(mut current) = self.current_theme.write() {
                *current = target_theme;
            }
            self.active_transition = None;
        }

        // Clear caches when switching themes
        self.clear_caches();

        // Update current variant
        if let Ok(mut current_var) = self.current_variant_internal.write() {
            *current_var = name.to_string();
        }

        // Notify all subscribers of theme change
        self.notify_theme_changed(name.to_string());

        Ok(())
    }

    /// Check if a transition is active and complete it if finished.
    pub fn update_transition(&mut self) {
        let should_finalize = if let Some(transition) = &self.active_transition {
            if let Ok(transition_guard) = transition.read() {
                transition_guard.is_complete()
            } else {
                false
            }
        } else {
            false
        };
        
        if should_finalize {
            // Transition complete, clear it
            // The target theme is already set, we just need to clear the transition
            self.active_transition = None;
            // Clear caches to force refresh with final theme
            self.clear_caches();
        }
    }

    /// Check if a transition is currently active.
    pub fn has_active_transition(&self) -> bool {
        self.active_transition.is_some()
    }

    /// Enable hot reload for theme configuration files.
    ///
    /// This will watch the specified config file and all referenced theme files
    /// for changes, and automatically reload the theme when files are modified.
    pub fn enable_hot_reload<P: AsRef<Path>>(
        &mut self,
        config_path: P,
        referenced_paths: Vec<PathBuf>,
    ) -> Result<(), ThemeError> {
        let config_path = config_path.as_ref().to_path_buf();
        let all_paths: Vec<PathBuf> = {
            let mut paths = vec![config_path.clone()];
            paths.extend(referenced_paths);
            paths
        };

        let (sender, receiver) = mpsc::channel();
        self.config_reload_sender = Some(sender.clone());
        self.config_reload_receiver = Some(receiver);
        self.config_path = Some(config_path.clone());

        // Setup file watcher
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            for path in &event.paths {
                                if let Err(_) = sender.send(path.clone()) {
                                    // Receiver dropped, stop watching
                                    break;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        log::warn!("File watcher error: {}", e);
                    },
                }
            },
            notify::Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| ThemeError::file_watcher_error(e))?;

        // Watch all paths
        for path in &all_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::NonRecursive)
                    .map_err(|e| ThemeError::file_watcher_error(e))?;
            }
        }

        self.file_watcher = Some(watcher);
        self.watched_paths = all_paths;

        Ok(())
    }

    /// Check for file changes and reload theme if needed.
    /// Returns true if a reload was triggered.
    pub fn check_and_reload(&mut self) -> Result<bool, ThemeError> {
        if let Some(receiver) = &mut self.config_reload_receiver {
            // Non-blocking check for file changes
            while let Ok(changed_path) = receiver.try_recv() {
                log::info!("Theme file changed: {:?}, reloading...", changed_path);
                
                // Reload from the main config file if it changed
                if let Some(config_path) = &self.config_path {
                    if changed_path == *config_path || self.watched_paths.contains(&changed_path) {
                        // Clone config_path to avoid borrow issues
                        let config_path = config_path.clone();
                        self.reload_from_file(&config_path)?;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Reload theme from a configuration file.
    pub fn reload_from_file<P: AsRef<Path>>(
        &mut self,
        config_path: P,
    ) -> Result<(), ThemeError> {
        use crate::config::ThemeConfig;
        
        let config = ThemeConfig::from_file(config_path)?;
        
        // Resolve the theme
        let theme = config.resolve_theme()?;
        
        // Determine theme name
        let theme_name = config.default_theme
            .as_ref()
            .map(|s| match s {
                crate::config::ThemeSource::Light => "light",
                crate::config::ThemeSource::Dark => "dark",
                crate::config::ThemeSource::Sweet => "sweet",
                crate::config::ThemeSource::Custom(name) => name.as_str(),
                crate::config::ThemeSource::File(_) => "custom",
            })
            .unwrap_or("sweet");

        // Add theme if not already present
        if !self.available_themes.contains_key(theme_name) {
            self.add_theme(theme_name, theme);
        }

        // Switch to the theme (with transition if enabled)
        use crate::transition::TransitionConfig as TransitionConfigType;
        let transition_config = if config.transitions.enabled {
            Some(TransitionConfigType::new(true, config.transitions.duration_ms))
        } else {
            None
        };
        
        self.switch_theme_with_transition(theme_name, transition_config.as_ref())?;

        Ok(())
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
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("sweet")
            .to_string()
    }

    /// Get all available theme variants.
    pub fn available_variants(&self) -> Vec<String> {
        self.available_themes.keys().cloned().collect()
    }

    /// Compute a cache key from widget ID and property.
    fn cache_key(id: &WidgetId, property: &ThemeProperty) -> u64 {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        property.hash(&mut hasher);
        hasher.finish()
    }

    /// Get a theme property with caching.
    pub fn get_property(&self, id: &WidgetId, property: &ThemeProperty) -> Option<vello::peniko::Color> {
        let cache_key = Self::cache_key(id, property);

        // Check if we're in a transition
        if let Some(transition) = &self.active_transition {
            if let Ok(transition_guard) = transition.read() {
                if let Some(color) = transition_guard.get_interpolated_color(id, property) {
                    return Some(color);
                }
                // Transition complete, fall through to normal lookup
            }
        }

        // Check cache first
        if let Ok(cache) = self.theme_cache.read() {
            if let Some(ThemeValue::Color(color)) = cache.get(&cache_key) {
                return Some(*color);
            }
        }

        // Get from current theme
        // Note: Theme::get_property requires ownership, so we clone here
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
                // Return cloned value (necessary for ThemeValue enum)
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
                // Return cloned value (necessary for ThemeValue enum)
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
