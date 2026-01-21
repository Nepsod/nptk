// SPDX-License-Identifier: LGPL-3.0-only

//! Shared utility functions for theme parsing and color handling.

use vello::peniko::Color;
use super::error::ThemeError;

/// Parse a hex color string with optional alpha channel.
///
/// Supports both RGB and RGBA formats:
/// - `#rrggbb` - 6 characters, opaque (alpha = 255)
/// - `#rrggbbaa` - 8 characters, with alpha channel (0-255)
///
/// Examples:
/// - `#ff0000` - Red (opaque)
/// - `#ff000080` - Red with 50% opacity (128/255)
/// - `#00000000` - Transparent black
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

/// Create a color with RGBA components (convenience wrapper).
///
/// This is a convenience function that wraps `Color::from_rgba8`.
pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba8(r, g, b, a)
}

/// Create a color with RGB components, defaulting alpha to 255 (opaque).
///
/// This is a convenience function that wraps `Color::from_rgb8`.
pub fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}
