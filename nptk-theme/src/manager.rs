use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeValue};
use crate::theme::{Theme, celeste::CelesteTheme, dark::DarkTheme};

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
    fn clone_theme(&self, _theme: &dyn Theme) -> Box<dyn Theme + Send + Sync> {
        // This is a simplified implementation - in practice, you'd need proper cloning
        // For now, we'll create new instances of known themes
        // TODO: Implement proper theme cloning based on theme type
        Box::new(CelesteTheme::light())
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
pub fn create_shared_theme_manager_with_theme(theme: Box<dyn Theme + Send + Sync>) -> SharedThemeManager {
    Arc::new(RwLock::new(ThemeManager::with_theme(theme)))
}
