use parley::fontique::{Blob, Collection, CollectionOptions, QueryFamily, QueryFont, SourceCache};
use vello::peniko::FontData;
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
    /// Create a new font context with system fonts loaded immediately.
    pub fn new() -> Self {
        let context = Self {
            collection: Arc::new(RwLock::new(Collection::new(CollectionOptions {
                system_fonts: true,
                ..Default::default()
            }))),
            source_cache: Arc::new(RwLock::new(SourceCache::new(Default::default()))),
        };

        context.discover_system_fonts();
        context
    }

    /// Discover and load system fonts into the collection.
    ///
    /// This triggers font discovery using fontique's fontconfig backend.
    fn discover_system_fonts(&self) {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();
        let _ = collection.query(&mut source_cache);
        log::debug!(
            "FontContext discovered {} font families",
            collection.family_names().count()
        );
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
    pub fn load(&mut self, name: impl ToString, font: FontData) {
        let font_name = name.to_string();
        let mut collection = self.collection.write().unwrap();

        let (data, _) = font.data.into_raw_parts();
        let result = collection.register_fonts(Blob::new(data), None);
        log::debug!("Loaded font '{}' with {} families", font_name, result.len());
    }

    /// Load a font from file asynchronously.
    pub async fn load_from_file_async<P: AsRef<std::path::Path>>(&mut self, name: impl ToString, path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let font_name = name.to_string();
        let font_data = smol::fs::read(path).await?;
        
        let mut collection = self.collection.write().unwrap();
        let result = collection.register_fonts(Blob::new(Arc::new(font_data)), None);
        log::debug!("Loaded font '{}' from file with {} families", font_name, result.len());
        
        Ok(())
    }

    /// Load multiple fonts from files asynchronously.
    pub async fn load_fonts_batch_async(&mut self, fonts: Vec<(String, std::path::PathBuf)>) -> Vec<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
        let mut results = Vec::new();
        
        for (name, path) in fonts {
            let result = self.load_from_file_async(name, path).await;
            results.push(result);
        }
        
        results
    }

    /// Get the default font.
    pub fn default_font(&self) -> Option<QueryFont> {
        self.query_with_families(
            [QueryFamily::Generic(
                parley::fontique::GenericFamily::SansSerif,
            )],
            "Default font",
        )
    }

    /// Get a font by name.
    pub fn get(&self, name: &str) -> Option<QueryFont> {
        self.query_with_families([QueryFamily::Named(name)], &format!("Font '{}'", name))
    }

    /// Create a query with the given families and execute it.
    ///
    /// This extracts the common pattern of locking, creating a query, and executing it.
    fn query_with_families(
        &self,
        families: [QueryFamily; 1],
        description: &str,
    ) -> Option<QueryFont> {
        let mut collection = self.collection.write().unwrap();
        let mut source_cache = self.source_cache.write().unwrap();

        let mut query = collection.query(&mut source_cache);
        query.set_families(families);

        self.execute_query(query, description)
    }

    /// Execute a font query and return the first match.
    ///
    /// This extracts the common query execution pattern used by both `default_font()` and `get()`.
    fn execute_query(
        &self,
        mut query: parley::fontique::Query<'_>,
        description: &str,
    ) -> Option<QueryFont> {
        let mut result = None;
        query.matches_with(|font| {
            result = Some(font.clone());
            parley::fontique::QueryStatus::Stop
        });

        if result.is_none() {
            log::warn!("{} not available", description);
        }

        result
    }
}

impl Default for FontContext {
    fn default() -> Self {
        Self::new()
    }
}
