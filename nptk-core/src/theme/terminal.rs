// SPDX-License-Identifier: LGPL-3.0-only

//! Terminal color scheme loading and management.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vello::peniko::Color;
use super::error::ThemeError;

/// Terminal color scheme (ANSI colors).
#[derive(Debug, Clone)]
pub struct TerminalColors {
    /// Show bold text as bright colors.
    pub show_bold_as_bright: bool,
    /// Primary background color.
    pub background: Color,
    /// Primary foreground color.
    pub foreground: Color,
    /// Normal colors (ANSI 0-7).
    pub normal: [Color; 8],
    /// Bright colors (ANSI 8-15).
    pub bright: [Color; 8],
}

impl TerminalColors {
    /// Create a new terminal color scheme with defaults.
    pub fn new() -> Self {
        Self {
            show_bold_as_bright: true,
            background: Color::BLACK,
            foreground: Color::WHITE,
            normal: [
                Color::BLACK,      // 0
                Color::from_rgb8(204, 0, 0),      // 1 Red
                Color::from_rgb8(62, 154, 6),     // 2 Green
                Color::from_rgb8(196, 160, 0),   // 3 Yellow
                Color::from_rgb8(52, 101, 164),  // 4 Blue
                Color::from_rgb8(117, 80, 123),  // 5 Magenta
                Color::from_rgb8(6, 152, 154),   // 6 Cyan
                Color::from_rgb8(238, 238, 238), // 7 White
            ],
            bright: [
                Color::from_rgb8(85, 87, 83),    // 8 BrightBlack
                Color::from_rgb8(239, 41, 41),  // 9 BrightRed
                Color::from_rgb8(138, 226, 52),  // 10 BrightGreen
                Color::from_rgb8(252, 233, 79),  // 11 BrightYellow
                Color::from_rgb8(114, 159, 207), // 12 BrightBlue
                Color::from_rgb8(173, 127, 168), // 13 BrightMagenta
                Color::from_rgb8(52, 226, 226), // 14 BrightCyan
                Color::WHITE,                     // 15 BrightWhite
            ],
        }
    }

    /// Parse a hex color string (e.g., "#ff0000" or "#ff0000ff").
    pub fn parse_hex_color(hex: &str) -> Result<Color, ThemeError> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            Ok(Color::from_rgb8(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            let a = u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| ThemeError::InvalidColor(hex.to_string()))?;
            Ok(Color::from_rgba8(r, g, b, a))
        } else {
            Err(ThemeError::InvalidColor(format!("Hex color must be 6 or 8 characters: {}", hex)))
        }
    }

    /// Load terminal colors from a TOML file.
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ThemeError> {
        let path = path.as_ref();
        let content = smol::fs::read_to_string(path)
            .await
            .map_err(|e| ThemeError::ReadError(path.to_path_buf(), e))?;

        Self::load_from_toml(&content, path)
    }

    /// Load terminal colors from TOML content.
    pub fn load_from_toml<P: AsRef<Path>>(content: &str, path: P) -> Result<Self, ThemeError> {
        let path = path.as_ref();
        let table: toml::Value = toml::from_str(content)
            .map_err(|e| ThemeError::ParseError(path.to_path_buf(), e.to_string()))?;

        let mut colors = Self::new();

        // Parse [Options] section
        if let Some(options) = table.get("Options").and_then(|v| v.as_table()) {
            if let Some(show_bold) = options.get("ShowBoldTextAsBright").and_then(|v| v.as_bool()) {
                colors.show_bold_as_bright = show_bold;
            }
        }

        // Parse [Primary] section
        if let Some(primary) = table.get("Primary").and_then(|v| v.as_table()) {
            if let Some(bg) = primary.get("Background").and_then(|v| v.as_str()) {
                colors.background = Self::parse_hex_color(bg)?;
            }
            if let Some(fg) = primary.get("Foreground").and_then(|v| v.as_str()) {
                colors.foreground = Self::parse_hex_color(fg)?;
            }
        }

        // Parse [Normal] section (ANSI 0-7)
        if let Some(normal) = table.get("Normal").and_then(|v| v.as_table()) {
            let color_names = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"];
            for (i, name) in color_names.iter().enumerate() {
                if let Some(color_str) = normal.get(*name).and_then(|v| v.as_str()) {
                    colors.normal[i] = Self::parse_hex_color(color_str)?;
                }
            }
        }

        // Parse [Bright] section (ANSI 8-15)
        if let Some(bright) = table.get("Bright").and_then(|v| v.as_table()) {
            let color_names = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"];
            for (i, name) in color_names.iter().enumerate() {
                if let Some(color_str) = bright.get(*name).and_then(|v| v.as_str()) {
                    colors.bright[i] = Self::parse_hex_color(color_str)?;
                }
            }
        }

        Ok(colors)
    }

    /// Get a normal color by ANSI index (0-7).
    pub fn normal(&self, index: usize) -> Option<Color> {
        self.normal.get(index).copied()
    }

    /// Get a bright color by ANSI index (8-15, mapped to 0-7).
    pub fn bright(&self, index: usize) -> Option<Color> {
        if index >= 8 && index < 16 {
            self.bright.get(index - 8).copied()
        } else {
            None
        }
    }
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self::new()
    }
}

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
