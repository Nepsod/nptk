//! Thumbnail provider implementation using the thumbnailify crate.

use crate::filesystem::entry::FileEntry;
use npio::service::filesystem::mime_detector::MimeDetector;
use crate::thumbnail::cache::{is_thumbnail_fresh, thumbnail_cache_path};
use crate::thumbnail::executor::ThumbnailExecutor;
use crate::thumbnail::{ThumbnailError, ThumbnailProvider};
use std::path::PathBuf;
use std::sync::Arc;

/// Thumbnail provider using the thumbnailify crate.
pub struct ThumbnailifyProvider {
    executor: Arc<ThumbnailExecutor>,
}

impl ThumbnailifyProvider {
    /// Get a receiver for thumbnail events.
    ///
    /// This allows subscribing to events when thumbnails are ready or fail.
    pub fn subscribe(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::thumbnail::events::ThumbnailEvent> {
        self.executor.subscribe()
    }
}

impl ThumbnailifyProvider {
    /// Create a new thumbnailify provider.
    pub fn new() -> Self {
        Self {
            executor: Arc::new(ThumbnailExecutor::new()),
        }
    }

    /// Check if a MIME type is supported for thumbnail generation.
    fn is_mime_supported(mime_type: &str) -> bool {
        mime_type.starts_with("image/")
            || mime_type.starts_with("video/")
            || mime_type == "application/pdf"
    }
}

impl Default for ThumbnailifyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbnailProvider for ThumbnailifyProvider {
    fn get_thumbnail(&self, entry: &FileEntry, size: u32) -> Option<PathBuf> {
        // Only support files
        if !entry.is_file() {
            return None;
        }

        // Check if file type is supported
        if !self.is_supported(entry) {
            return None;
        }

        // Get cache path
        let thumbnail_path = thumbnail_cache_path(entry, size);

        // Check if thumbnail exists and is fresh
        if thumbnail_path.exists() && is_thumbnail_fresh(&thumbnail_path, &entry.path) {
            log::debug!("Thumbnail cache hit for {:?}", entry.path);
            return Some(thumbnail_path);
        }

        log::debug!("Thumbnail cache miss for {:?}", entry.path);
        None
    }

    fn request_thumbnail(&self, entry: &FileEntry, size: u32) -> Result<(), ThumbnailError> {
        // Only support files
        if !entry.is_file() {
            return Err(ThumbnailError::UnsupportedFileType(
                "Only files are supported for thumbnail generation".to_string(),
            ));
        }

        // Check if file type is supported
        if !self.is_supported(entry) {
            let mime_type = entry.metadata.mime_type.as_deref().unwrap_or("unknown");
            return Err(ThumbnailError::UnsupportedFileType(format!(
                "MIME type '{}' is not supported",
                mime_type
            )));
        }

        // Queue thumbnail generation
        self.executor
            .request_thumbnail(entry.clone(), size)
            .map_err(|e| ThumbnailError::Unknown(format!("Failed to queue thumbnail: {}", e)))?;

        log::debug!("Thumbnail generation queued for {:?}", entry.path);
        Ok(())
    }

    fn is_supported(&self, entry: &FileEntry) -> bool {
        // Only support files
        if !entry.is_file() {
            return false;
        }

        // Check MIME type
        if let Some(ref mime_type) = entry.metadata.mime_type {
            return Self::is_mime_supported(mime_type);
        }

        // Try to detect MIME type if not available
        if let Some(mime_type) = smol::block_on(MimeDetector::detect_mime_type(&entry.path)) {
            return Self::is_mime_supported(&mime_type);
        }

        false
    }
}
