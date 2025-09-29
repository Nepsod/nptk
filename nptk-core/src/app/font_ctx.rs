use fontique::{
    Collection, CollectionOptions, QueryFamily, QueryFont, SourceCache,
    FontStyle, FontWeight, FontWidth, Attributes, Blob
};
use peniko::Font;
use read_fonts::{FontRef, TableProvider};
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
    fallback_list: Arc<RwLock<Vec<String>>>,
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
            fallback_list: Arc::new(RwLock::new(Vec::new())),
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
            fallback_list: Arc::new(RwLock::new(Vec::new())),
        }
    }


    /// Get a reference to the underlying `fontique` collection.
    /// 
    /// Note: This returns a read lock guard, so the caller must handle the lock properly.
    pub fn collection(&self) -> std::sync::RwLockReadGuard<'_, Collection> {
        self.collection.read().unwrap()
    }

    /// Selects the best font that matches the query.
    pub fn select_best<'a>(
        &mut self,
        families: impl IntoIterator<Item = QueryFamily<'a>>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontWidth,
    ) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        
        let mut fontique_query = collection.query(&mut source_cache);
        fontique_query.set_families(families);
        let attributes = Attributes {
            style,
            weight,
            width: stretch,
        };
        fontique_query.set_attributes(attributes);
        
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

    /// Selects the best font for a specific character, using the fallback list if necessary.
    pub fn select_for_char(&mut self, ch: char) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        
        // Helper closure to find a font that supports the character.
        let find_font_for_char = |query: &mut fontique::Query, c: char| -> Option<QueryFont> {
            let mut result = None;
            query.matches_with(|font| {
                if let Ok(file) = FontRef::new(font.blob.data()) {
                    if let Ok(cmap) = file.cmap() {
                        if cmap.map_codepoint(c).is_some() {
                            result = Some(font.clone());
                            return fontique::QueryStatus::Stop;
                        }
                    }
                }
                fontique::QueryStatus::Continue
            });
            result
        };

        let mut query = collection.query(&mut source_cache);

        // 1. Try a comprehensive list of generic families.
        query.set_families([
            QueryFamily::Generic(fontique::GenericFamily::SansSerif),
            QueryFamily::Generic(fontique::GenericFamily::Serif),
            QueryFamily::Generic(fontique::GenericFamily::Monospace),
            QueryFamily::Generic(fontique::GenericFamily::Cursive),
            QueryFamily::Generic(fontique::GenericFamily::Fantasy),
            // "SystemUi" can be a good catch-all for UI symbols.
            QueryFamily::Generic(fontique::GenericFamily::SystemUi),
        ]);
        if let Some(font) = find_font_for_char(&mut query, ch) {
            return Some(font);
        }

        // 2. If no font is found, try the user-defined fallback list.
        let fallback_list = self.fallback_list.read().unwrap();
        if !fallback_list.is_empty() {
            let families = fallback_list.iter().map(|name| QueryFamily::Named(name));
            query.set_families(families);
            if let Some(font) = find_font_for_char(&mut query, ch) {
                return Some(font);
            }
        }

        // 3. As a last resort, clear the families to search everything.
        query.set_families(std::iter::empty::<QueryFamily>());
        if let Some(font) = find_font_for_char(&mut query, ch) {
            return Some(font);
        }
        
        log::warn!("No suitable font found for character '{}'", ch);
        None
    }

    /// Get a font family by name.
    pub fn get_family<'a>(&self, name: &'a str) -> Option<QueryFamily<'a>> {
        Some(QueryFamily::Named(name))
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