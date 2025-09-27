use fontique::{
    Collection, CollectionOptions, Query, QueryFamily, QueryFont
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
        // TODO: Implement fontique query when API is available
        log::warn!("Font selection not fully implemented - using fontconfig fallback");
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
        // TODO: Implement fontique character selection when API is available
        log::warn!("Character font selection not fully implemented - using fontconfig fallback");
        None
    }

    /// Get a font family by name.
    pub fn get_family(&self, _name: &str) -> Option<QueryFamily> {
        // TODO: Implement family lookup using fontique API
        None
    }

    /// Load a font into the collection.
    pub fn load(&mut self, _name: impl ToString, _font: Font) {
        // TODO: Implement font loading using fontique API
    }

    /// Load a system font into the collection.
    pub fn load_system(&mut self, _name: impl ToString, _postscript_name: impl ToString) {
        // TODO: Implement system font loading using fontique API
    }

    /// Get the default font.
    pub fn default_font(&self) -> Option<QueryFont> {
        // For now, return None until we properly implement fontique integration
        // The fontique API is complex and needs proper integration
        log::warn!("Default font not yet implemented - fontique API integration needed");
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
        // TODO: Implement fontique font lookup when API is available
        log::warn!("Font lookup not fully implemented for: {}", name);
        None
    }

    /// Convert a fontconfig font match to a fontique QueryFont.
    /// This is a bridge between the two font systems.
    fn convert_fontconfig_to_query_font(&self, font_match: &FontMatch) -> Option<QueryFont> {
        // For now, we'll create a minimal QueryFont
        // This will need to be enhanced when fontique API is fully available
        log::debug!("Converting fontconfig font match {:?} to QueryFont", font_match.id);
        
        // TODO: Implement proper conversion when fontique API is available
        // For now, return None to indicate conversion is not yet implemented
        None
    }
}

impl Default for FontContext {
    fn default() -> Self {
        Self::new()
    }
}