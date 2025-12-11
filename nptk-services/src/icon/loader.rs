//! Icon loader for PNG, SVG, and XPM files.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::filesystem::io_uring;
use crate::icon::cache::CachedIcon;
use crate::icon::error::IconError;

/// Icon loader.
pub struct IconLoader;

impl IconLoader {
    /// Create a new icon loader.
    pub fn new() -> Self {
        Self
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, IconError> {
        // If we're already inside a Tokio runtime, avoid blocking it; use std::fs.
        if tokio::runtime::Handle::try_current().is_ok() {
            return fs::read(path)
                .map_err(|e| IconError::InvalidFormat(format!("Failed to load bytes: {}", e)));
        }
        // Outside a runtime, try io_uring, fall back to std on error.
        match tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| handle.block_on(async { io_uring::read(path).await.ok() }))
        {
            Some(bytes) => Ok(bytes),
            None => fs::read(path)
                .map_err(|e| IconError::InvalidFormat(format!("Failed to load bytes: {}", e))),
        }
    }

    fn read_string(&self, path: &Path) -> Result<String, IconError> {
        // If we're already inside a Tokio runtime, avoid blocking it; use std::fs.
        if tokio::runtime::Handle::try_current().is_ok() {
            return fs::read_to_string(path)
                .map_err(|e| IconError::InvalidFormat(format!("Failed to load SVG: {}", e)));
        }
        // Outside a runtime, try io_uring, fall back to std on error.
        match tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| handle.block_on(async { io_uring::read_to_string(path).await.ok() }))
        {
            Some(text) => Ok(text),
            None => fs::read_to_string(path)
                .map_err(|e| IconError::InvalidFormat(format!("Failed to load SVG: {}", e))),
        }
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
        let svg_content = self.read_string(path)?;
        Ok(CachedIcon::Svg(Arc::new(svg_content)))
    }

    /// Load a PNG icon.
    fn load_png(&self, path: &Path) -> Result<CachedIcon, IconError> {
        let bytes = self.read_bytes(path)?;
        let img = image::load_from_memory(&bytes)
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
        let bytes = self.read_bytes(path)?;
        let img = image::load_from_memory(&bytes)
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
