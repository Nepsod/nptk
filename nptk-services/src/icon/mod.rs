//! XDG Icon Theme System
//!
//! This module provides a complete implementation of the XDG Icon Theme Specification,
//! including theme parsing, icon lookup with inheritance, caching, and loading.

mod cache;
mod error;
mod loader;
mod lookup;
mod theme;

pub use cache::CachedIcon;
pub use error::IconError;
pub use loader::IconLoader;
pub use lookup::IconLookup;
pub use theme::{DirectoryType, IconContext, IconDirectory, IconTheme};

use crate::filesystem::entry::FileEntry;
use crate::filesystem::icon::{IconProvider, MimeIconProvider};

/// Icon registry - main public API for the icon system.
pub struct IconRegistry {
    /// Current theme name.
    theme: String,
    /// Icon lookup system.
    lookup: IconLookup,
    /// Icon cache.
    cache: cache::IconCache,
    /// Icon loader.
    loader: IconLoader,
    /// MIME icon provider for mapping files to icon names.
    mime_provider: MimeIconProvider,
}

impl IconRegistry {
    /// Create a new icon registry with the default theme "Sweet-Purple".
    pub fn new() -> Result<Self, IconError> {
        Self::with_theme(None)
    }

    /// Create a new icon registry with a specific theme.
    pub fn with_theme(theme: Option<String>) -> Result<Self, IconError> {
        let theme_name = theme.unwrap_or_else(|| "Sweet-Purple".to_string());
        
        // Try to load the theme to verify it exists
        let lookup = IconLookup::new();
        lookup.load_theme(&theme_name)?;

        Ok(Self {
            theme: theme_name,
            lookup,
            cache: cache::IconCache::new(),
            loader: IconLoader::new(),
            mime_provider: MimeIconProvider::new(),
        })
    }

    /// Set the current theme.
    pub fn set_theme(&mut self, theme: String) -> Result<(), IconError> {
        // Verify theme exists
        self.lookup.load_theme(&theme)?;
        self.theme = theme;
        // Clear cache when theme changes
        self.cache.clear();
        Ok(())
    }

    /// Get the current theme name.
    pub fn theme(&self) -> &str {
        &self.theme
    }

    /// Get an icon by name and size.
    pub fn get_icon(&self, icon_name: &str, size: u32) -> Option<CachedIcon> {
        // Check cache first
        if let Some(cached) = self.cache.get(icon_name, size) {
            return Some(cached);
        }

        // Determine context based on icon name patterns
        let context = self.guess_context(icon_name);

        // Lookup icon path
        let icon_path = self.lookup.lookup_icon(icon_name, size, context, &self.theme)?;

        // Load icon
        let cached_icon = self.loader.load_icon(&icon_path).ok()?;

        // Cache it
        self.cache.put(icon_name.to_string(), size, cached_icon.clone());

        Some(cached_icon)
    }

    /// Get an icon for a file entry with fallback chain.
    pub fn get_file_icon(&self, entry: &FileEntry, size: u32) -> Option<CachedIcon> {
        // Get icon candidates from MIME provider
        let icon_data = self.mime_provider.get_icon(entry)?;
        log::debug!("IconRegistry: Looking up icons {:?} for file '{}' (MIME: {:?})", 
            icon_data.names, 
            entry.name,
            entry.metadata.mime_type
        );
        
        // Try specific icons in order
        for name in &icon_data.names {
            if let Some(icon) = self.get_icon(name, size) {
                return Some(icon);
            }
        }
        
        // Try generic category for each candidate
        for name in &icon_data.names {
            let generic_name = self.get_generic_icon_name(name);
            if let Some(icon) = self.get_icon(&generic_name, size) {
                log::debug!("IconRegistry: Found generic icon '{}' from '{}'", generic_name, name);
                return Some(icon);
            }
        }
        
        // Additional fallbacks for specific cases
        // For media-floppy, try drive-harddisk as fallback
        if icon_data.names.iter().any(|n| n == "media-floppy") {
            if let Some(icon) = self.get_icon("drive-harddisk", size) {
                log::debug!("IconRegistry: Using drive-harddisk as fallback for media-floppy");
                return Some(icon);
            }
        }
        
        // For application-toml, try text-x-generic as fallback (TOML is text-like)
        if icon_data.names.iter().any(|n| n == "application-toml") {
            if let Some(icon) = self.get_icon("text-x-generic", size) {
                log::debug!("IconRegistry: Using text-x-generic as fallback for application-toml");
                return Some(icon);
            }
        }
        
        // Final fallback: text-x-generic or unknown
        self.get_icon("text-x-generic", size)
            .or_else(|| self.get_icon("unknown", size))
    }

    /// Get generic icon name from specific icon name.
    /// 
    /// Maps specific icons to their generic category:
    /// - text-x-toml -> text-x-generic
    /// - application-pdf -> application-x-generic
    /// - image-png -> image-x-generic
    /// - media-floppy -> drive-removable-media (fallback for floppy)
    fn get_generic_icon_name(&self, icon_name: &str) -> String {
        if icon_name.starts_with("text-") && icon_name != "text-x-generic" {
            "text-x-generic".to_string()
        } else if icon_name.starts_with("application-") && !icon_name.ends_with("-generic") {
            // Try application-x-generic first, fallback to application-generic
            if icon_name.contains("-x-") {
                "application-x-generic".to_string()
            } else {
                "application-x-generic".to_string()
            }
        } else if icon_name.starts_with("image-") && icon_name != "image-x-generic" {
            "image-x-generic".to_string()
        } else if icon_name.starts_with("video-") && icon_name != "video-x-generic" {
            "video-x-generic".to_string()
        } else if icon_name.starts_with("audio-") && icon_name != "audio-x-generic" {
            "audio-x-generic".to_string()
        } else if icon_name == "media-floppy" {
            // Fallback for floppy disk: try drive-removable-media, then drive-harddisk
            "drive-removable-media".to_string()
        } else if icon_name.starts_with("media-") {
            // For other media icons, try drive-harddisk as fallback
            "drive-harddisk".to_string()
        } else {
            // Already generic or unknown, return as-is
            icon_name.to_string()
        }
    }

    /// Guess icon context from icon name.
    fn guess_context(&self, icon_name: &str) -> IconContext {
        if icon_name.starts_with("folder") || icon_name.contains("directory") {
            IconContext::Places
        } else if icon_name.starts_with("media-") || icon_name.starts_with("drive-") {
            // Media and drive icons are in Devices context
            IconContext::Devices
        } else if icon_name.starts_with("text-") {
            IconContext::Mimetypes
        } else if icon_name.starts_with("image-") {
            IconContext::Mimetypes
        } else if icon_name.starts_with("application-") {
            IconContext::Mimetypes
        } else if icon_name.starts_with("video-") {
            IconContext::Mimetypes
        } else if icon_name.starts_with("audio-") {
            IconContext::Mimetypes
        } else {
            IconContext::Unknown
        }
    }
}

impl Default for IconRegistry {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to hicolor if Sweet-Purple not found
            Self::with_theme(Some("hicolor".to_string()))
                .unwrap_or_else(|_| {
                    // Last resort: create with Adwaita
                    Self::with_theme(Some("Adwaita".to_string()))
                        .expect("Failed to initialize icon registry")
                })
        })
    }
}

