use fontique::{Blob, Collection, CollectionOptions, QueryFamily, QueryFont, SourceCache};
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
        let context = Self {
            collection: Arc::new(RwLock::new(Collection::new(CollectionOptions {
                system_fonts: true,
                ..Default::default()
            }))),
            source_cache: Arc::new(RwLock::new(SourceCache::new(Default::default()))),
        };
        
        // Trigger font discovery immediately
        {
            let mut collection = context.collection.write().unwrap();
            let mut source_cache = context.source_cache.write().unwrap();
            let _ = collection.query(&mut source_cache); // This triggers font discovery
            log::debug!("FontContext::new_with_system_fonts() loaded {} font families", collection.family_names().count());
        }
        
        
        context
    }


    /// Create a parley FontContext from our fontique collection
    /// This bridges our custom font management with Parley's text layout engine
    pub fn create_parley_font_context(&self) -> parley::FontContext {
        let collection = self.collection.read().unwrap();
        let source_cache = self.source_cache.read().unwrap();
        
        parley::FontContext {
            collection: collection.clone(),
            source_cache: source_cache.clone(),
        }
    }


    /// Load a font into the collection.
    pub fn load(&mut self, name: impl ToString, font: Font) {
        let name = name.to_string();
        let mut collection = self.collection.write().unwrap();
        
        let (data, _) = font.data.into_raw_parts();
        let result = collection.register_fonts(Blob::new(data), None);
        log::debug!("Loaded font '{}' with {} families", name, result.len());
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

    
}

impl Default for FontContext {
    fn default() -> Self {
        Self::new()
    }
}