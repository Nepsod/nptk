// SPDX-License-Identifier: LGPL-3.0-only
//! Thumbnail provider implementation using `thumbnailify`.
//!
//! This provider uses the `thumbnailify` crate to generate thumbnails
//! for various file formats. It supports local files and uses the
//! ThumbnailExecutor for background generation.

use crate::filesystem::entry::FileEntry;
use crate::thumbnail::cache::{is_thumbnail_fresh, thumbnail_cache_path};
use crate::thumbnail::executor::ThumbnailExecutor;
use crate::thumbnail::{ThumbnailError, ThumbnailProvider};
use std::sync::Arc;

/// A thumbnail provider that uses `thumbnailify`.
pub struct ThumbnailifyProvider {
    executor: Arc<ThumbnailExecutor>,
}

impl ThumbnailifyProvider {
    /// Create a new `ThumbnailifyProvider`.
    pub fn new() -> Self {
        Self {
            executor: Arc::new(ThumbnailExecutor::new()),
        }
    }

    /// Subscribe to thumbnail events.
    pub fn subscribe(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::thumbnail::events::ThumbnailEvent> {
        self.executor.subscribe()
    }
}

impl Default for ThumbnailifyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbnailProvider for ThumbnailifyProvider {
    fn is_supported(&self, entry: &FileEntry) -> bool {
        // Only local files are supported
        if entry.path.to_string_lossy().starts_with("http") {
            return false;
        }

        // Check if thumbnailify supports this MIME type
        if let Some(mime_type) = &entry.metadata.mime_type {
            // This is a rough check, thumbnailify supports many image formats
            mime_type.starts_with("image/")
                || mime_type.starts_with("video/")
                || mime_type == "application/pdf"
        } else {
            // Fallback to extension check
            if let Some(ext) = entry.path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                matches!(
                    ext.as_str(),
                    "png"
                        | "jpg"
                        | "jpeg"
                        | "gif"
                        | "bmp"
                        | "webp"
                        | "svg"
                        | "mp4"
                        | "mkv"
                        | "avi"
                        | "mov"
                        | "pdf"
                )
            } else {
                false
            }
        }
    }

    fn get_thumbnail(&self, entry: &FileEntry, size: u32) -> Option<std::path::PathBuf> {
        if !self.is_supported(entry) {
            return None;
        }

        let thumbnail_path = thumbnail_cache_path(entry, size);

        // Blocking on async operation inside synchronous method
        // Use smol::block_on as it's lightweight and works within tokio context usually
        if smol::block_on(async {
            // Use metadata check instead of exists() + is_fresh which does double work
            // is_thumbnail_fresh handles existence check safely
            is_thumbnail_fresh(&thumbnail_path, &entry.path).await
        }) {
            Some(thumbnail_path)
        } else {
            None
        }
    }

    fn request_thumbnail(&self, entry: &FileEntry, size: u32) -> Result<(), ThumbnailError> {
        if !self.is_supported(entry) {
            return Err(ThumbnailError::UnsupportedFileType(
                entry.path.to_string_lossy().to_string(),
            ));
        }

        self.executor
            .request_thumbnail(entry.clone(), size)
    }
}
