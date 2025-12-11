//! Icon lookup system with search paths and inheritance.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::icon::error::IconError;
use crate::icon::theme::{DirectoryType, IconContext, IconDirectory, IconTheme};

/// Icon lookup system.
pub struct IconLookup {
    /// Cache of loaded themes.
    theme_cache: Arc<Mutex<HashMap<String, IconTheme>>>,
    /// Search paths for icon themes.
    search_paths: Vec<PathBuf>,
}

impl IconLookup {
    /// Create a new icon lookup system.
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // User-specific paths
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            search_paths.push(home.join(".icons"));
            search_paths.push(home.join(".local/share/icons"));
        }

        // System-wide paths
        search_paths.push(PathBuf::from("/usr/share/icons"));
        search_paths.push(PathBuf::from("/usr/share/pixmaps"));

        Self {
            theme_cache: Arc::new(Mutex::new(HashMap::new())),
            search_paths,
        }
    }

    /// Load a theme (with caching).
    pub fn load_theme(&self, theme_name: &str) -> Result<IconTheme, IconError> {
        let mut cache = self.theme_cache.lock().unwrap();

        if let Some(theme) = cache.get(theme_name) {
            return Ok(theme.clone());
        }

        // Try to find theme in search paths
        for search_path in &self.search_paths {
            let theme_path = search_path.join(theme_name);
            if theme_path.exists() && theme_path.is_dir() {
                match IconTheme::load(theme_name, theme_path) {
                    Ok(theme) => {
                        let theme_clone = theme.clone();
                        cache.insert(theme_name.to_string(), theme);
                        return Ok(theme_clone);
                    },
                    Err(_) => continue,
                }
            }
        }

        Err(IconError::ThemeNotFound(theme_name.to_string()))
    }

    /// Lookup an icon in a theme and its inherited themes.
    pub fn lookup_icon(
        &self,
        icon_name: &str,
        size: u32,
        context: IconContext,
        theme_name: &str,
    ) -> Option<PathBuf> {
        log::debug!(
            "IconLookup: Looking for icon '{}' (size: {}, context: {:?}) in theme '{}'",
            icon_name,
            size,
            context,
            theme_name
        );

        // Try current theme
        if let Ok(theme) = self.load_theme(theme_name) {
            if let Some(path) = self.lookup_in_theme(&theme, icon_name, size, context) {
                log::debug!("IconLookup: Found icon '{}' at {:?}", icon_name, path);
                return Some(path);
            }

            // Try inherited themes
            for inherited in &theme.inherits {
                log::debug!("IconLookup: Trying inherited theme '{}'", inherited);
                if let Ok(inherited_theme) = self.load_theme(inherited) {
                    if let Some(path) =
                        self.lookup_in_theme(&inherited_theme, icon_name, size, context)
                    {
                        log::debug!(
                            "IconLookup: Found icon '{}' in inherited theme '{}' at {:?}",
                            icon_name,
                            inherited,
                            path
                        );
                        return Some(path);
                    }
                }
            }
        }

        // Fallback to hicolor
        if theme_name != "hicolor" {
            log::debug!("IconLookup: Falling back to hicolor theme");
            if let Some(path) = self.lookup_icon(icon_name, size, context, "hicolor") {
                return Some(path);
            }
        }

        log::debug!("IconLookup: Icon '{}' not found in any theme", icon_name);
        None
    }

    /// Lookup icon in a specific theme.
    fn lookup_in_theme(
        &self,
        theme: &IconTheme,
        icon_name: &str,
        size: u32,
        context: IconContext,
    ) -> Option<PathBuf> {
        // Find best matching directory
        let best_dir = match self.find_best_directory(&theme.directories, size, context) {
            Some(dir) => dir,
            None => {
                log::debug!(
                    "IconLookup: No matching directory found for size {} and context {:?}",
                    size,
                    context
                );
                return None;
            },
        };

        let dir_path = theme.directory_path(&best_dir.name);
        log::debug!(
            "IconLookup: Searching in directory '{}' at {:?}",
            best_dir.name,
            dir_path
        );

        // Try different file extensions in order of preference
        let extensions = ["svg", "png", "xpm"];
        for ext in &extensions {
            let icon_path = dir_path.join(format!("{}.{}", icon_name, ext));
            if icon_path.exists() {
                log::debug!("IconLookup: Found icon file at {:?}", icon_path);
                return Some(icon_path);
            }
        }

        // If not found in best directory, try all directories (some themes organize differently)
        log::debug!(
            "IconLookup: Icon '{}' not found in directory '{}', trying all directories",
            icon_name,
            best_dir.name
        );
        for dir in &theme.directories {
            if dir.name == best_dir.name {
                continue; // Already tried this one
            }
            let dir_path = theme.directory_path(&dir.name);
            for ext in &extensions {
                let icon_path = dir_path.join(format!("{}.{}", icon_name, ext));
                if icon_path.exists() {
                    log::debug!(
                        "IconLookup: Found icon file at {:?} in directory '{}'",
                        icon_path,
                        dir.name
                    );
                    return Some(icon_path);
                }
            }
        }

        log::debug!(
            "IconLookup: Icon '{}' not found in theme '{}'",
            icon_name,
            theme.name
        );
        None
    }

    /// Find the best matching directory for a given size and context.
    fn find_best_directory<'a>(
        &self,
        directories: &'a [IconDirectory],
        size: u32,
        context: IconContext,
    ) -> Option<&'a IconDirectory> {
        // First try: exact context match
        let mut candidates: Vec<&IconDirectory> = directories
            .iter()
            .filter(|d| d.context == context)
            .collect();

        // Second try: Unknown context (many themes use this)
        if candidates.is_empty() {
            candidates = directories
                .iter()
                .filter(|d| d.context == IconContext::Unknown)
                .collect();
        }

        // Third try: Mimetypes context (for file type icons)
        if candidates.is_empty() && context == IconContext::Mimetypes {
            candidates = directories
                .iter()
                .filter(|d| d.context == IconContext::Mimetypes)
                .collect();
        }

        // Final fallback: any directory
        if candidates.is_empty() {
            candidates = directories.iter().collect();
        }

        // Sort by how well they match the requested size
        candidates.sort_by_key(|d| {
            let score: u32 = match d.directory_type {
                DirectoryType::Fixed => {
                    let diff = if d.size > size {
                        d.size - size
                    } else {
                        size - d.size
                    };
                    diff
                },
                DirectoryType::Scalable => {
                    let min = d.min_size.unwrap_or(16);
                    let max = d.max_size.unwrap_or(256);
                    if size >= min && size <= max {
                        0 // Perfect match
                    } else if size < min {
                        min - size
                    } else {
                        size - max
                    }
                },
                DirectoryType::Threshold => {
                    let threshold = d.threshold.unwrap_or(2);
                    let diff = if d.size > size {
                        d.size - size
                    } else {
                        size - d.size
                    };
                    if diff <= threshold {
                        0
                    } else {
                        diff
                    }
                },
            };
            score
        });

        candidates.first().copied()
    }
}

impl Default for IconLookup {
    fn default() -> Self {
        Self::new()
    }
}
