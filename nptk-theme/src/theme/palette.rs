use vello::peniko::Color;

mod serde_color_inner {
    use serde::{Deserializer, Serializer, Deserialize};
    use vello::peniko::Color;
    
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
            Ok(Color::from_rgb8(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| "Invalid hex color")?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| "Invalid hex color")?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| "Invalid hex color")?;
            let a = u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| "Invalid hex color")?;
            Ok(Color::from_rgba8(r, g, b, a))
        } else {
            Err("Hex color must be 6 or 8 characters".to_string())
        }
    }
}

use serde_color_inner as serde_color;

/// Shared palette description for themes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThemePalette {
    /// Core accent color used across widgets.
    #[serde(with = "serde_color")]
    pub primary: Color,
    /// Lighter variant of the primary color.
    #[serde(with = "serde_color")]
    pub primary_light: Color,
    /// Darker variant of the primary color.
    #[serde(with = "serde_color")]
    pub primary_dark: Color,
    /// Secondary accent color.
    #[serde(with = "serde_color")]
    pub accent: Color,
    /// Default background color.
    #[serde(with = "serde_color")]
    pub background: Color,
    /// Alternate background used for raised surfaces.
    #[serde(with = "serde_color")]
    pub background_alt: Color,
    /// Elevated background color for popovers.
    #[serde(with = "serde_color")]
    pub background_elevated: Color,
    /// Main text color.
    #[serde(with = "serde_color")]
    pub text: Color,
    /// Muted text color for secondary labels.
    #[serde(with = "serde_color")]
    pub text_muted: Color,
    /// Border color for separators and outlines.
    #[serde(with = "serde_color")]
    pub border: Color,
    /// Selection highlight color.
    #[serde(with = "serde_color")]
    pub selection: Color,
}

/// Trait for types capable of exposing a [ThemePalette].
pub trait ProvidesPalette {
    /// Obtain the palette reference.
    fn palette(&self) -> &ThemePalette;
}

impl ThemePalette {
    /// Standard palette for the dark built-in theme.
    pub fn dark() -> Self {
        Self {
            primary: Color::from_rgb8(100, 150, 255),
            primary_light: Color::from_rgb8(120, 170, 255),
            primary_dark: Color::from_rgb8(80, 130, 235),
            accent: Color::from_rgb8(80, 130, 235),
            background: Color::from_rgb8(30, 30, 30),
            background_alt: Color::from_rgb8(40, 40, 40),
            background_elevated: Color::from_rgb8(50, 50, 50),
            text: Color::from_rgb8(220, 220, 220),
            text_muted: Color::from_rgb8(140, 140, 140),
            border: Color::from_rgb8(80, 80, 80),
            selection: Color::from_rgb8(100, 150, 255),
        }
    }

    /// Vibrant palette for the sweet theme.
    pub fn sweet() -> Self {
        Self {
            primary: Color::from_rgb8(197, 14, 210),
            primary_light: Color::from_rgb8(254, 207, 14),
            primary_dark: Color::from_rgb8(157, 51, 213),
            accent: Color::from_rgb8(0, 232, 198),
            background: Color::from_rgb8(22, 25, 37),
            background_alt: Color::from_rgb8(30, 34, 51),
            background_elevated: Color::from_rgb8(24, 27, 40),
            text: Color::from_rgb8(211, 218, 227),
            text_muted: Color::from_rgb8(102, 106, 115),
            border: Color::from_rgb8(102, 106, 115),
            selection: Color::from_rgb8(197, 14, 210),
        }
    }

    /// Palette used by the light Celeste theme.
    pub fn celeste_light() -> Self {
        Self {
            primary: Color::from_rgb8(150, 170, 250),
            primary_light: Color::from_rgb8(170, 170, 250),
            primary_dark: Color::from_rgb8(120, 140, 220),
            accent: Color::from_rgb8(100, 150, 255),
            background: Color::from_rgb8(255, 255, 255),
            background_alt: Color::from_rgb8(245, 245, 245),
            background_elevated: Color::from_rgb8(230, 230, 230),
            text: Color::from_rgb8(0, 0, 0),
            text_muted: Color::from_rgb8(150, 150, 150),
            border: Color::from_rgb8(200, 200, 200),
            selection: Color::from_rgb8(100, 150, 255),
        }
    }
}
