// SPDX-License-Identifier: LGPL-3.0-only

//! Terminal color scheme resolution and built-in schemes.
//!
//! This module provides resolution logic for terminal color schemes:
//! - Built-in schemes (Default, Sweet)
//! - Custom schemes from XDG directories
//! - Theme-embedded schemes (via "theme" keyword)

use std::path::{Path, PathBuf};
use vello::peniko::Color;
use super::super::error::ThemeError;
use super::colors::TerminalColors;

/// Built-in terminal color schemes.
pub struct BuiltinTerminalSchemes;

impl BuiltinTerminalSchemes {
    /// List of built-in scheme names.
    pub const BUILTIN_NAMES: &'static [&'static str] = &["Default", "Sweet"];

    /// Check if a scheme name is built-in.
    pub fn is_builtin(name: &str) -> bool {
        Self::BUILTIN_NAMES.contains(&name)
    }

    /// Get a built-in terminal color scheme.
    pub fn get(name: &str) -> Option<TerminalColors> {
        match name {
            "Default" => Some(TerminalColors::new()),
            "Sweet" => Some(Self::sweet()),
            _ => None,
        }
    }

    /// Create the Sweet terminal color scheme.
    fn sweet() -> TerminalColors {
        TerminalColors {
            show_bold_as_bright: true,
            background: Color::from_rgb8(22, 25, 37),
            foreground: Color::from_rgb8(211, 218, 227),
            normal: [
                Color::from_rgb8(22, 25, 37),    // Black
                Color::from_rgb8(251, 43, 44),   // Red
                Color::from_rgb8(48, 211, 58),   // Green
                Color::from_rgb8(254, 207, 14),  // Yellow
                Color::from_rgb8(16, 106, 254),  // Blue
                Color::from_rgb8(197, 14, 210),  // Magenta
                Color::from_rgb8(0, 232, 198),   // Cyan
                Color::from_rgb8(211, 218, 227), // White
            ],
            bright: [
                Color::from_rgb8(47, 52, 63),    // BrightBlack
                Color::from_rgb8(251, 43, 44),   // BrightRed
                Color::from_rgb8(48, 211, 58),   // BrightGreen
                Color::from_rgb8(254, 207, 14), // BrightYellow
                Color::from_rgb8(16, 106, 254),  // BrightBlue
                Color::from_rgb8(197, 14, 210),  // BrightMagenta
                Color::from_rgb8(0, 232, 198),   // BrightCyan
                Color::from_rgb8(254, 254, 254), // BrightWhite
            ],
        }
    }
}

/// Resolve terminal colors based on the resolution logic.
pub async fn resolve_terminal_colors(
    value: &str,
    theme_dir: Option<&Path>,
    override_builtin: bool,
) -> Result<TerminalColors, ThemeError> {
    // 1. If value == "theme", load from theme directory
    if value == "theme" {
        if let Some(dir) = theme_dir {
            let term_colors_path = dir.join("term-colors.toml");
            if term_colors_path.exists() {
                return TerminalColors::load_from_file(&term_colors_path).await;
            }
        }
        return Err(ThemeError::TerminalColorsNotFound(
            "theme".to_string(),
        ));
    }

    // 2. Check if it's a built-in scheme
    if BuiltinTerminalSchemes::is_builtin(value) && !override_builtin {
        if let Some(colors) = BuiltinTerminalSchemes::get(value) {
            return Ok(colors);
        }
    }

    // 3. Try to load as custom theme from XDG directories
    let search_paths = get_terminal_colors_search_paths(value);
    for path in search_paths {
        if path.exists() {
            return TerminalColors::load_from_file(&path).await;
        }
    }

    // 4. Fallback: try built-in if override was set
    if override_builtin {
        if let Some(colors) = BuiltinTerminalSchemes::get(value) {
            return Ok(colors);
        }
    }

    Err(ThemeError::TerminalColorsNotFound(value.to_string()))
}

/// Get search paths for terminal color schemes.
fn get_terminal_colors_search_paths(scheme_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Search in theme directories first
    if let Ok(home) = std::env::var("HOME") {
        // ~/themes/<scheme-name>/nptk-0/term-colors.toml
        paths.push(PathBuf::from(&home).join("themes").join(scheme_name).join("nptk-0").join("term-colors.toml"));
        
        // ~/.local/share/themes/<scheme-name>/nptk-0/term-colors.toml
        paths.push(PathBuf::from(&home).join(".local").join("share").join("themes").join(scheme_name).join("nptk-0").join("term-colors.toml"));
    }

    // /usr/share/themes/<scheme-name>/nptk-0/term-colors.toml
    paths.push(PathBuf::from("/usr/share/themes").join(scheme_name).join("nptk-0").join("term-colors.toml"));

    // Fallback paths (if not in theme directory)
    if let Ok(home) = std::env::var("HOME") {
        // ~/.local/share/nptk/term-colors/<scheme-name>.toml
        paths.push(PathBuf::from(&home).join(".local").join("share").join("nptk").join("term-colors").join(format!("{}.toml", scheme_name)));
    }

    // /usr/share/nptk/term-colors/<scheme-name>.toml
    paths.push(PathBuf::from("/usr/share/nptk").join("term-colors").join(format!("{}.toml", scheme_name)));

    paths
}
