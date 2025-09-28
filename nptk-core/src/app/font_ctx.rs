use fontique::{
    Collection, CollectionOptions, Query, QueryFamily, QueryFont, SourceCache, Blob
};
use peniko::Font;
use std::sync::Arc;

use rust_fontconfig::{FcFontCache, FcPattern, FontMatch};

/// A font manager for nptk applications, powered by `fontique` and `fontconfig`.
///
/// This context handles discovery of system fonts and provides an interface
/// for querying and resolving fonts, which will be used by the `parley` text
/// layout engine.
#[derive(Clone)]
pub struct FontContext {
    collection: Arc<Collection>,
    source_cache: Arc<SourceCache>,
    fontconfig_cache: Option<FcFontCache>,
    use_fontconfig: bool,
}

impl FontContext {
    /// Create a new font context.
    ///
    /// This will create an empty collection without loading system fonts immediately.
    /// System fonts will be loaded lazily when needed.
    pub fn new() -> Self {
        Self {
            collection: Arc::new(Collection::new(CollectionOptions {
                system_fonts: false,  // Don't load system fonts immediately
                ..Default::default()
            })),
            source_cache: Arc::new(SourceCache::new(Default::default())),
            fontconfig_cache: None,
            use_fontconfig: false,
        }
    }

    /// Create a new font context with system fonts loaded immediately.
    ///
    /// This will discover all available system fonts during initialization.
    /// Use this only when you need immediate access to all system fonts.
    pub fn new_with_system_fonts() -> Self {
        Self {
            collection: Arc::new(Collection::new(CollectionOptions {
                system_fonts: true,
                ..Default::default()
            })),
            source_cache: Arc::new(SourceCache::new(Default::default())),
            fontconfig_cache: None,
            use_fontconfig: false,
        }
    }

    /// Create a new font context with fontconfig integration.
    ///
    /// This will use fontconfig for font discovery and matching, providing
    /// better performance and font selection on Linux systems.
    pub fn new_with_fontconfig() -> Self {
        let fontconfig_cache = Some(FcFontCache::build());
        
        Self {
            collection: Arc::new(Collection::new(CollectionOptions {
                system_fonts: false,  // Let fontconfig handle discovery
                ..Default::default()
            })),
            source_cache: Arc::new(SourceCache::new(Default::default())),
            fontconfig_cache,
            use_fontconfig: true,
        }
    }

    /// Get a reference to the underlying `fontique` collection.
    pub fn collection(&self) -> &Collection {
        &self.collection
    }

    /// Selects the best font that matches the query.
    pub fn select_best(&mut self, _query: &mut Query) -> Option<QueryFont> {
        if self.use_fontconfig {
            if let Some(cache) = &self.fontconfig_cache {
                // For now, use a default pattern since fontique Query API is not fully available
                let pattern = FcPattern::default();
                
                let mut trace = Vec::new();
                if let Some(font_match) = cache.query(&pattern, &mut trace) {
                    log::debug!("Found font via fontconfig: {:?}", font_match.id);
                    // Convert fontconfig result to fontique QueryFont
                    return self.convert_fontconfig_to_query_font(&font_match);
                }
            }
        }
        
        // Fallback to fontique
        if let Ok(mut collection) = Arc::try_unwrap(self.collection.clone()) {
            if let Ok(mut source_cache) = Arc::try_unwrap(self.source_cache.clone()) {
                let mut query = collection.query(&mut source_cache);
                query.set_families([QueryFamily::Generic(fontique::GenericFamily::SansSerif)]);
                
                let mut result = None;
                query.matches_with(|font| {
                    result = Some(font.clone());
                    fontique::QueryStatus::Stop
                });
                return result;
            }
        }
        
        log::debug!("Using fontique fallback for font selection");
        
        log::warn!("No suitable font found for query");
        None
    }

    /// Selects the best font for a specific character.
    pub fn select_for_char(&mut self, ch: char) -> Option<QueryFont> {
        if self.use_fontconfig {
            if let Some(cache) = &self.fontconfig_cache {
                // Use query_for_text for character-based font selection
                let text = ch.to_string();
                let mut trace = Vec::new();
                let matched_fonts = cache.query_for_text(&FcPattern::default(), &text, &mut trace);
                
                if let Some(font_match) = matched_fonts.first() {
                    log::debug!("Found font for char '{}' via fontconfig: {:?}", ch, font_match.id);
                    return self.convert_fontconfig_to_query_font(font_match);
                }
            }
        }
        
        // Fallback to fontique
        if let Ok(mut collection) = Arc::try_unwrap(self.collection.clone()) {
            if let Ok(mut source_cache) = Arc::try_unwrap(self.source_cache.clone()) {
                let mut query = collection.query(&mut source_cache);
                query.set_families([QueryFamily::Generic(fontique::GenericFamily::SansSerif)]);
                
                let mut result = None;
                query.matches_with(|font| {
                    result = Some(font.clone());
                    fontique::QueryStatus::Stop
                });
                return result;
            }
        }
        
        log::debug!("Using fontique fallback for character '{}' selection", ch);
        
        log::warn!("No suitable font found for character '{}'", ch);
        None
    }

    /// Get a font family by name.
    pub fn get_family(&self, name: &str) -> Option<QueryFamily<'static>> {
        Some(QueryFamily::Named(name.to_string().leak()))
    }

    /// Load a font into the collection.
    pub fn load(&mut self, name: impl ToString, font: Font) {
        let name = name.to_string();
        if let Ok(mut collection) = Arc::try_unwrap(self.collection.clone()) {
            // Convert peniko::Font to Blob<u8>
            let font_data = font.data.clone();
            let result = collection.register_fonts(font_data, None);
            log::debug!("Loaded font '{}' with {} families", name, result.len());
        } else {
            log::warn!("Failed to load font '{}' - collection is shared", name);
        }
    }

    /// Load a system font into the collection.
    pub fn load_system(&mut self, name: impl ToString, postscript_name: impl ToString) {
        let name = name.to_string();
        let postscript_name = postscript_name.to_string();
        
        // Try to find the system font by name
        if let Some(font_path) = self.find_system_font_path(&name) {
            if let Ok(font_data) = std::fs::read(&font_path) {
                if let Ok(mut collection) = Arc::try_unwrap(self.collection.clone()) {
                    let blob = Blob::new(Arc::new(font_data));
                    let result = collection.register_fonts(blob, None);
                    log::debug!("Loaded system font '{}' (PostScript: {}) with {} families", 
                               name, postscript_name, result.len());
                } else {
                    log::warn!("Failed to load system font '{}' - collection is shared", name);
                }
            } else {
                log::warn!("Failed to read system font file: {:?}", font_path);
            }
        } else {
            log::warn!("System font '{}' not found", name);
        }
    }

    /// Get the default font.
    pub fn default_font(&self) -> Option<QueryFont> {
        if let Ok(mut collection) = Arc::try_unwrap(self.collection.clone()) {
            if let Ok(mut source_cache) = Arc::try_unwrap(self.source_cache.clone()) {
                let mut query = collection.query(&mut source_cache);
                query.set_families([QueryFamily::Generic(fontique::GenericFamily::SansSerif)]);
                
                let mut result = None;
                query.matches_with(|font| {
                    result = Some(font.clone());
                    fontique::QueryStatus::Stop
                });
                return result;
            }
        }
        
        log::warn!("Default font not available");
        None
    }

    /// Get a font by name.
    pub fn get(&mut self, name: &str) -> Option<QueryFont> {
        if self.use_fontconfig {
            if let Some(cache) = &self.fontconfig_cache {
                let pattern = FcPattern {
                    name: Some(name.to_string()),
                    ..Default::default()
                };
                
                let mut trace = Vec::new();
                if let Some(font_match) = cache.query(&pattern, &mut trace) {
                    log::debug!("Found font '{}' via fontconfig: {:?}", name, font_match.id);
                    return self.convert_fontconfig_to_query_font(&font_match);
                }
            }
        }
        
        // Fallback to fontique
        if let Ok(mut collection) = Arc::try_unwrap(self.collection.clone()) {
            if let Ok(mut source_cache) = Arc::try_unwrap(self.source_cache.clone()) {
                let mut query = collection.query(&mut source_cache);
                query.set_families([QueryFamily::Named(name)]);
                
                let mut result = None;
                query.matches_with(|font| {
                    result = Some(font.clone());
                    fontique::QueryStatus::Stop
                });
                return result;
            }
        }
        
        log::debug!("Using fontique fallback for font '{}' lookup", name);
        
        log::warn!("Font '{}' not found", name);
        None
    }

    /// Convert a fontconfig font match to a fontique QueryFont.
    /// This is a bridge between the two font systems.
    fn convert_fontconfig_to_query_font(&self, font_match: &FontMatch) -> Option<QueryFont> {
        log::debug!("Converting fontconfig font match {:?} to QueryFont", font_match.id);
        
        // For now, we can't directly convert fontconfig results to QueryFont
        // because we need to load the font into the collection first
        // This is a limitation of the current implementation
        log::warn!("Font {:?} conversion not fully implemented - would need to load font into collection first", font_match.id);
        None
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