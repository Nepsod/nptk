// SPDX-License-Identifier: LGPL-3.0-only

//! Terminal color scheme structure and parsing.
//!
//! The `TerminalColors` struct represents a complete ANSI terminal color scheme
//! with support for normal colors (0-7), bright colors (8-15), and primary
//! background/foreground colors.

use std::path::{Path, PathBuf};
use vello::peniko::Color;
use super::super::error::ThemeError;

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

    /// Parse a hex color string with optional alpha channel.
    ///
    /// This is a convenience wrapper around `super::super::util::parse_hex_color`
    /// for backward compatibility.
    pub fn parse_hex_color(hex: &str) -> Result<Color, ThemeError> {
        super::super::util::parse_hex_color(hex)
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
