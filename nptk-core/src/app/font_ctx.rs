use fontique::{
    Collection, CollectionOptions, Query, QueryFamily, QueryFont, SourceCache, Blob
};
use peniko::Font;
use std::sync::{Arc, RwLock};

/// A font manager for nptk applications, powered by `fontique` with system font support.
///
/// This context handles discovery of system fonts using fontique's built-in
/// fontconfig backend (always enabled on Linux) and provides an interface for 
/// querying and resolving fonts, which will be used by the `parley` text layout engine.
#[derive(Clone)]
pub struct FontContext {
    collection: Arc<RwLock<Collection>>,
    source_cache: Arc<RwLock<SourceCache>>,
}

impl FontContext {
    /// Create a new font context.
    ///
    /// This will create an empty collection without loading system fonts immediately.
    /// System fonts will be loaded lazily when needed.
    pub fn new() -> Self {
        Self {
            collection: Arc::new(RwLock::new(Collection::new(CollectionOptions {
                system_fonts: false,  // Don't load system fonts immediately
                ..Default::default()
            }))),
            source_cache: Arc::new(RwLock::new(SourceCache::new(Default::default()))),
        }
    }

    /// Create a new font context with system fonts loaded immediately.
    ///
    /// This will discover all available system fonts during initialization using
    /// fontique's built-in fontconfig backend (on Linux). Use this only when you
    /// need immediate access to all system fonts.
    pub fn new_with_system_fonts() -> Self {
        Self {
            collection: Arc::new(RwLock::new(Collection::new(CollectionOptions {
                system_fonts: true,
                ..Default::default()
            }))),
            source_cache: Arc::new(RwLock::new(SourceCache::new(Default::default()))),
        }
    }


    /// Get a reference to the underlying `fontique` collection.
    /// 
    /// Note: This returns a read lock guard, so the caller must handle the lock properly.
    pub fn collection(&self) -> std::sync::RwLockReadGuard<'_, Collection> {
        self.collection.read().unwrap()
    }

    /// Selects the best font that matches the query.
    pub fn select_best(&mut self, _query: &mut Query) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        
        let mut fontique_query = collection.query(&mut source_cache);
        fontique_query.set_families([QueryFamily::Generic(fontique::GenericFamily::SansSerif)]);
        
        let mut result = None;
        fontique_query.matches_with(|font| {
            result = Some(font.clone());
            fontique::QueryStatus::Stop
        });
        
        if result.is_some() {
            result
        } else {
            log::warn!("No suitable font found for query");
            None
        }
    }

    /// Selects the best font for a specific character.
    pub fn select_for_char(&mut self, ch: char) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        
        let mut query = collection.query(&mut source_cache);
        query.set_families([QueryFamily::Generic(fontique::GenericFamily::SansSerif)]);
        
        let mut result = None;
        query.matches_with(|font| {
            result = Some(font.clone());
            fontique::QueryStatus::Stop
        });
        
        if result.is_some() {
            result
        } else {
            log::warn!("No suitable font found for character '{}'", ch);
            None
        }
    }

    /// Get a font family by name.
    pub fn get_family(&self, name: &str) -> Option<QueryFamily<'static>> {
        Some(QueryFamily::Named(name.to_string().leak()))
    }

    /// Load a font into the collection.
    pub fn load(&mut self, name: impl ToString, font: Font) {
        let name = name.to_string();
        let mut collection = self.collection.write().unwrap();
        
        // Convert peniko::Font to Blob<u8>
        let font_data = font.data.clone();
        let result = collection.register_fonts(font_data, None);
        log::debug!("Loaded font '{}' with {} families", name, result.len());
    }

    /// Load a system font into the collection.
    pub fn load_system(&mut self, name: impl ToString, postscript_name: impl ToString) {
        let name = name.to_string();
        let postscript_name = postscript_name.to_string();
        
        // Try to find the system font by name
        if let Some(font_path) = self.find_system_font_path(&name) {
            if let Ok(font_data) = std::fs::read(&font_path) {
                let mut collection = self.collection.write().unwrap();
                let blob = Blob::new(Arc::new(font_data));
                let result = collection.register_fonts(blob, None);
                log::debug!("Loaded system font '{}' (PostScript: {}) with {} families", 
                           name, postscript_name, result.len());
            } else {
                log::warn!("Failed to read system font file: {:?}", font_path);
            }
        } else {
            log::warn!("System font '{}' not found", name);
        }
    }

    /// Get the default font.
    pub fn default_font(&self) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        
        let mut query = collection.query(&mut source_cache);
        query.set_families([QueryFamily::Generic(fontique::GenericFamily::SansSerif)]);
        
        let mut result = None;
        query.matches_with(|font| {
            result = Some(font.clone());
            fontique::QueryStatus::Stop
        });
        
        if result.is_some() {
            result
        } else {
            log::warn!("Default font not available");
            None
        }
    }

    /// Get a font by name.
    pub fn get(&mut self, name: &str) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        
        let mut query = collection.query(&mut source_cache);
        query.set_families([QueryFamily::Named(name)]);
        
        let mut result = None;
        query.matches_with(|font| {
            result = Some(font.clone());
            fontique::QueryStatus::Stop
        });
        
        if result.is_some() {
            result
        } else {
            log::warn!("Font '{}' not found", name);
            None
        }
    }

    
    /// Helper method to find system font path by name.
    fn find_system_font_path(&self, name: &str) -> Option<std::path::PathBuf> {
        // Common system font directories
        let font_dirs = [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/System/Library/Fonts",
            "/Library/Fonts",
            "~/.fonts",
            "~/.local/share/fonts",
        ];
        
        for dir in &font_dirs {
            let expanded_dir = if dir.starts_with("~") {
                format!("{}/{}", std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()), &dir[2..])
            } else {
                dir.to_string()
            };
            if let Ok(entries) = std::fs::read_dir(&expanded_dir) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.to_lowercase().contains(&name.to_lowercase()) {
                            if let Some(ext) = entry.path().extension() {
                                if matches!(ext.to_str(), Some("ttf") | Some("otf") | Some("woff") | Some("woff2")) {
                                    return Some(entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
}

impl Default for FontContext {
    fn default() -> Self {
        Self::new()
    }
}