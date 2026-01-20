// SPDX-License-Identifier: LGPL-3.0-only

//! Theme resolver for built-in vs custom themes.

use std::path::PathBuf;
use super::error::ThemeError;
use super::loader::ThemeLoader;
use super::builtin::create_sweet_theme;
use super::Theme;

/// Theme resolver for resolving built-in and custom themes.
pub struct ThemeResolver;

impl ThemeResolver {
    /// Resolve a theme by name.
    ///
    /// Returns a Theme if found, or an error if not found.
    pub async fn resolve(theme_name: &str) -> Result<Theme, ThemeError> {
        // Check if it's a built-in theme
        if Self::is_builtin(theme_name) {
            return Self::get_builtin(theme_name);
        }

        // Try to load as custom theme
        Self::load_custom(theme_name).await
    }

    /// Check if a theme name is built-in.
    pub fn is_builtin(name: &str) -> bool {
        matches!(name, "Sweet" | "sweet")
    }

    /// Get a built-in theme.
    pub fn get_builtin(name: &str) -> Result<Theme, ThemeError> {
        match name {
            "Sweet" | "sweet" => Ok(create_sweet_theme()),
            _ => Err(ThemeError::InvalidThemeName(name.to_string())),
        }
    }

    /// Load a custom theme from XDG directories.
    pub async fn load_custom(theme_name: &str) -> Result<Theme, ThemeError> {
        if let Some(theme_dir) = ThemeLoader::find_theme_directory(theme_name) {
            let theme_file = theme_dir.join("theme.toml");
            ThemeLoader::load_from_file(&theme_file).await
        } else {
            Err(ThemeError::ThemeDirectoryNotFound(
                PathBuf::from(theme_name),
            ))
        }
    }

    /// List available themes (built-in + custom).
    pub async fn list_themes() -> Vec<String> {
        let mut themes = Vec::new();

        // Add built-in themes
        themes.push("Sweet".to_string());

        // TODO: Scan XDG directories for custom themes
        // This would require async directory scanning

        themes
    }
}
