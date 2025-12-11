//! Icon loader for PNG, SVG, and XPM files.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::icon::cache::CachedIcon;
use crate::icon::error::IconError;

/// Icon loader.
pub struct IconLoader;

impl IconLoader {
    /// Create a new icon loader.
    pub fn new() -> Self {
        Self
    }

    /// Load an icon from a file path.
    pub fn load_icon(&self, path: &Path) -> Result<CachedIcon, IconError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "svg" => self.load_svg(path),
            "png" => self.load_png(path),
            "xpm" => self.load_xpm(path),
            _ => Err(IconError::InvalidFormat(format!(
                "Unsupported icon format: {}",
                extension
            ))),
        }
    }

    /// Load an SVG icon.
    fn load_svg(&self, path: &Path) -> Result<CachedIcon, IconError> {
        let svg_content = fs::read_to_string(path)?;
        Ok(CachedIcon::Svg(Arc::new(svg_content)))
    }

    /// Load a PNG icon.
    fn load_png(&self, path: &Path) -> Result<CachedIcon, IconError> {
        let img = image::open(path)
            .map_err(|e| IconError::InvalidFormat(format!("Failed to load PNG: {}", e)))?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let data = rgba.into_raw();

        Ok(CachedIcon::Image {
            data: Arc::new(data),
            width,
            height,
        })
    }

    /// Load an XPM icon (convert to PNG-like format).
    fn load_xpm(&self, path: &Path) -> Result<CachedIcon, IconError> {
        // XPM is a text-based format, but for simplicity, we'll try to load it as image
        // The image crate should handle XPM
        let img = image::open(path)
            .map_err(|e| IconError::InvalidFormat(format!("Failed to load XPM: {}", e)))?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let data = rgba.into_raw();

        Ok(CachedIcon::Image {
            data: Arc::new(data),
            width,
            height,
        })
    }
}

impl Default for IconLoader {
    fn default() -> Self {
        Self::new()
    }
}
