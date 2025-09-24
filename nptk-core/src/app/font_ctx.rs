use indexmap::IndexMap;
use peniko::{Blob, Font};
use std::sync::Arc;
use font_kit::family_name::FamilyName;
use font_kit::properties::{Properties, Weight, Style, Stretch};
use std::collections::HashMap;

/// A font manager for nptk applications.
///
/// Can be used to load and access in-memory fonts or by system source.
///
/// If the default `include-noto-sans` feature is enabled, the default font is set to [Noto Sans](https://fonts.google.com/specimen/Noto+Sans).
#[derive(Clone, Debug)]
pub struct FontContext {
    default: String,
    fonts: IndexMap<String, Font>,
    // Enhanced font discovery capabilities
    font_database: Option<HashMap<String, Vec<FontInfo>>>,
    fallback_chains: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct FontInfo {
    /// PostScript name of the font
    pub postscript_name: String,
    /// Family name of the font
    pub family_name: String,
    /// Font properties (weight, style, stretch)
    pub properties: Properties,
}

impl FontContext {
    /// Create a new font context with the given default font name.
    ///
    /// Make sure to load the default font via [FontContext::load],
    /// before passing this context to the application runner.
    pub fn new(default: String) -> Self {
        Self {
            default,
            fonts: IndexMap::new(),
            font_database: None,
            fallback_chains: None,
        }
    }

    /// Create a new font context with enhanced dynamic font discovery.
    ///
    /// This will automatically discover system fonts and set up intelligent defaults.
    pub fn new_with_discovery() -> Self {
        let mut ctx = Self {
            default: "Default".to_string(),
            fonts: IndexMap::new(),
            font_database: None,
            fallback_chains: None,
        };
        
        // Discover system fonts and set up intelligent defaults
        ctx.discover_system_fonts();
        ctx.setup_intelligent_default();
        
        ctx
    }

    /// Loads a font with a custom name into the font context.
    ///
    /// If the font with the same name already exists, it will be overwritten and the old font will be returned.
    pub fn load(&mut self, name: impl ToString, font: Font) -> Option<Font> {
        self.fonts.insert(name.to_string(), font)
    }

    /// Loads a system font into the font context.
    /// The provided name must match the postscript name of the font.
    ///
    /// If a font with the same name is already loaded, it will be overwritten and the old font will be returned.
    ///
    /// Returns `None` if the font could not be loaded.
    ///
    /// **NOTE:** Not every postscript font is available on every system.
    pub fn load_system(
        &mut self,
        name: impl ToString,
        postscript_name: impl ToString,
    ) -> Option<()> {
        log::debug!("Loading system font: {}", postscript_name.to_string());

        let font = font_kit::source::SystemSource::new()
            .select_by_postscript_name(postscript_name.to_string().as_str())
            .ok()?
            .load()
            .ok()?
            .copy_font_data()?;

        self.load(name, Font::new(Blob::new(font), 0));

        Some(())
    }

    /// Set the default font.
    ///
    /// **NOTE:** The font must be loaded before usage with [FontContext::load].
    pub fn set_default_font(&mut self, name: impl ToString) {
        self.default = name.to_string();
    }

    /// Get a font by a specified name. Returns [None] if the font could not be found.
    pub fn get(&self, name: impl ToString) -> Option<Font> {
        self.fonts.get(&name.to_string()).cloned()
    }

    /// Removes a font by the given name and returns it or [None] if the font could not be found.
    pub fn remove(&mut self, name: impl ToString) -> Option<Font> {
        self.fonts.swap_remove(&name.to_string())
    }

    /// Returns the default font. [Roboto](https://fonts.google.com/specimen/Roboto) by default.
    pub fn default_font(&self) -> &Font {
        self.fonts
            .get(&self.default)
            .expect("Default font not found. Please load one via `FontContext::load`.")
    }

    /// Discover all available system fonts and build a font database.
    ///
    /// This method scans the system for all available fonts and builds an internal
    /// database for intelligent font selection and fallback chains.
    pub fn discover_system_fonts(&mut self) {
        log::info!("Discovering system fonts...");
        
        let system_source = font_kit::source::SystemSource::new();
        match system_source.all_fonts() {
            Ok(all_fonts) => {
                let mut font_database = HashMap::new();
                let mut font_count = 0;
                
                for font_handle in all_fonts {
                    // Load the font to get its properties
                    if let Ok(font) = font_handle.load() {
                        let family_name = font.family_name();
                        let postscript_name = font.postscript_name();
                        let properties = font.properties();
                        
                        let font_info = FontInfo {
                            postscript_name: postscript_name.clone().unwrap_or_default(),
                            family_name: family_name.clone(),
                            properties,
                        };
                        
                        font_database
                            .entry(family_name)
                            .or_insert_with(Vec::new)
                            .push(font_info);
                        
                        font_count += 1;
                    }
                }
                
                self.font_database = Some(font_database);
                log::info!("Discovered {} fonts across {} families", 
                          font_count, self.font_database.as_ref().unwrap().len());
                
                // Build intelligent fallback chains
                self.build_fallback_chains();
            }
            Err(e) => {
                log::warn!("Failed to discover system fonts: {}", e);
                self.font_database = None;
            }
        }
    }

    /// Build intelligent fallback chains based on discovered fonts.
    fn build_fallback_chains(&mut self) {
        let font_database = match &self.font_database {
            Some(db) => db,
            None => return,
        };

        // Define preferred font families in order of preference
        let preferred_families = [
            "DejaVu Sans",
            "Liberation Sans", 
            "Ubuntu",
            "Cantarell",
            "Noto Sans",
            "FreeSans",
            "Arial",
            "Helvetica",
            "Segoe UI",      // Windows
            "SF Pro Display", // macOS
            "Roboto",        // Android
        ];

        // Build sans-serif fallback chain
        let mut sans_serif_chain = Vec::new();
        for family in &preferred_families {
            if font_database.contains_key(*family) {
                sans_serif_chain.push(family.to_string());
            }
        }
        
        // Add any other discovered sans-serif fonts
        for family in font_database.keys() {
            if !sans_serif_chain.contains(family) && 
               self.is_sans_serif_family(family) {
                sans_serif_chain.push(family.clone());
            }
        }
        
        let mut fallback_chains = HashMap::new();
        fallback_chains.insert("sans-serif".to_string(), sans_serif_chain);
        
        // Build monospace fallback chain
        let mut monospace_chain = Vec::new();
        let monospace_families = [
            "DejaVu Sans Mono",
            "Liberation Mono",
            "Ubuntu Mono", 
            "Noto Sans Mono",
            "FreeMono",
            "Courier New",
            "Monaco",
            "Consolas",
        ];
        
        for family in &monospace_families {
            if font_database.contains_key(*family) {
                monospace_chain.push(family.to_string());
            }
        }
        
        fallback_chains.insert("monospace".to_string(), monospace_chain);
        self.fallback_chains = Some(fallback_chains);
        
        log::debug!("Built fallback chains: {:?}", self.fallback_chains);
    }

    /// Check if a font family is likely a sans-serif font.
    fn is_sans_serif_family(&self, family: &str) -> bool {
        let family_lower = family.to_lowercase();
        family_lower.contains("sans") || 
        family_lower.contains("arial") ||
        family_lower.contains("helvetica") ||
        family_lower.contains("ubuntu") ||
        family_lower.contains("cantarell") ||
        family_lower.contains("noto") ||
        family_lower.contains("liberation")
    }

    /// Set up the default font using intelligent selection.
    fn setup_intelligent_default(&mut self) {
        // Try to find the best default font using CSS-style matching
        let properties = Properties {
            weight: Weight::NORMAL,
            style: Style::Normal,
            stretch: Stretch::NORMAL,
        };

        // Try to find a good default font
        if let Some(font_handle) = self.find_best_font(&["sans-serif"], &properties) {
            if let Ok(font) = font_handle.load() {
                let family_name = font.family_name();
                let postscript_name = font.postscript_name();
                
                // Load the font
                if let Some(font_data) = font.copy_font_data() {
                    self.fonts.insert(family_name.clone(), Font::new(Blob::new(font_data), 0));
                    self.default = family_name.clone();
                    log::info!("Selected default font: {} ({:?})", family_name, postscript_name);
                    return;
                }
            }
        }

        // Fallback to hardcoded approach if intelligent selection fails
        log::warn!("Intelligent font selection failed, falling back to hardcoded list");
        self.fallback_to_hardcoded_fonts();
    }

    /// Find the best font using CSS-style font matching.
    fn find_best_font(&self, families: &[&str], properties: &Properties) -> Option<font_kit::handle::Handle> {
        // Convert family names to font-kit format
        let family_names: Vec<FamilyName> = families
            .iter()
            .map(|&name| {
                if name == "sans-serif" {
                    FamilyName::SansSerif
                } else if name == "serif" {
                    FamilyName::Serif
                } else if name == "monospace" {
                    FamilyName::Monospace
                } else {
                    FamilyName::Title(name.to_string())
                }
            })
            .collect();

        // Use font-kit's CSS-compliant font matching
        let system_source = font_kit::source::SystemSource::new();
        system_source
            .select_best_match(&family_names, properties)
            .ok()
    }


    /// Fallback to the original hardcoded approach.
    fn fallback_to_hardcoded_fonts(&mut self) {
        let common_fonts = [
            ("DejaVu Sans", "DejaVuSans"),
            ("Liberation Sans", "LiberationSans"),
            ("Ubuntu", "Ubuntu"),
            ("Cantarell", "Cantarell"),
            ("Noto Sans", "NotoSans"),
            ("FreeSans", "FreeSans"),
            ("Arial", "Arial"),
            ("Helvetica", "Helvetica"),
        ];
        
        for (display_name, postscript_name) in &common_fonts {
            if self.load_system(display_name, postscript_name).is_some() {
                self.default = display_name.to_string();
                log::info!("Fallback: Selected font {}", display_name);
                return;
            }
        }
        
        // Ultimate fallback to embedded font
        self.load_embedded_fallback();
    }

    /// Load embedded fallback font.
    fn load_embedded_fallback(&mut self) {
        let minimal_font_data = include_bytes!("../NotoSans.ttf");
        self.fonts.insert("Fallback".to_string(), 
                         Font::new(Blob::new(Arc::new(*minimal_font_data)), 0));
        self.default = "Fallback".to_string();
        log::warn!("Using embedded fallback font");
    }

    /// Get available fonts for a family.
    pub fn get_available_fonts(&self, family: &str) -> Vec<&FontInfo> {
        self.font_database
            .as_ref()
            .and_then(|db| db.get(family))
            .map(|fonts| fonts.iter().collect())
            .unwrap_or_default()
    }

    /// Get fallback chain for a font family.
    pub fn get_fallback_chain(&self, family: &str) -> Vec<String> {
        self.fallback_chains
            .as_ref()
            .and_then(|chains| chains.get(family))
            .cloned()
            .unwrap_or_default()
    }

    /// Find font by family and properties.
    pub fn find_font_by_properties(&self, family: &str, weight: Weight, style: Style) -> Option<Font> {
        let properties = Properties {
            weight,
            style,
            stretch: Stretch::NORMAL,
        };

        let font_handle = self.find_best_font(&[family], &properties)?;
        if let Ok(font) = font_handle.load() {
            if let Some(font_data) = font.copy_font_data() {
                return Some(Font::new(Blob::new(font_data), 0));
            }
        }
        None
    }

    /// Get all discovered font families.
    pub fn get_font_families(&self) -> Vec<String> {
        self.font_database
            .as_ref()
            .map(|db| db.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the name of the default font.
    pub fn get_default_font_name(&self) -> &str {
        &self.default
    }
}

#[cfg(feature = "include-noto-sans")]
impl Default for FontContext {
    fn default() -> Self {
        let mut ctx = FontContext::new("Noto Sans".to_string());

        ctx.load(
            "Noto Sans",
            Font::new(Blob::new(Arc::new(crate::DEFAULT_FONT)), 0),
        );

        ctx
    }
}

// Universal Default implementation that works without the include-noto-sans feature
// Now uses dynamic font discovery instead of hardcoded lists
#[cfg(not(feature = "include-noto-sans"))]
impl Default for FontContext {
    fn default() -> Self {
        // Use enhanced dynamic font discovery by default
        Self::new_with_discovery()
    }
}
