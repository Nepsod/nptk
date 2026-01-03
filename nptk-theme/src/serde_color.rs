//! Custom serialization helpers for vello::peniko::Color

use serde::{Deserializer, Serializer};
use vello::peniko::Color;

/// Serialize a Color as a hex string.
pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let components = color.components;
    let r = (components[0] * 255.0) as u8;
    let g = (components[1] * 255.0) as u8;
    let b = (components[2] * 255.0) as u8;
    let a = (components[3] * 255.0) as u8;
    let hex = if a == 255 {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
    };
    serializer.serialize_str(&hex)
}

/// Deserialize a Color from a hex string.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let hex = String::deserialize(deserializer)?;
    parse_hex_color(&hex).map_err(Error::custom)
}

fn parse_hex_color(hex: &str) -> Result<Color, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| "Invalid hex color")?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| "Invalid hex color")?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| "Invalid hex color")?;
        Ok(Color::rgb8(r, g, b))
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| "Invalid hex color")?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| "Invalid hex color")?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| "Invalid hex color")?;
        let a = u8::from_str_radix(&hex[6..8], 16)
            .map_err(|_| "Invalid hex color")?;
        Ok(Color::rgba8(r, g, b, a))
    } else {
        Err("Hex color must be 6 or 8 characters".to_string())
    }
}
