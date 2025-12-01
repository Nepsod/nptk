//! Thumbnail generation and caching system.
//!
//! This module provides a pluggable thumbnail system that generates and caches
//! thumbnails for images, videos, and PDFs. It follows the freedesktop.org
//! Thumbnail Managing Standard for cache paths and naming.

use std::path::PathBuf;
use crate::filesystem::entry::FileEntry;

pub mod cache;
pub mod error;
pub mod executor;
pub mod thumbnailify_provider;
pub mod events;

pub use error::ThumbnailError;
pub use thumbnailify_provider::ThumbnailifyProvider;

/// Trait for thumbnail providers.
///
/// Thumbnail providers are responsible for generating and caching thumbnails
/// for supported file types. They should follow the freedesktop.org
/// Thumbnail Managing Standard for cache paths and naming.
pub trait ThumbnailProvider: Send + Sync + std::any::Any {
    /// Returns a path to a cached thumbnail for the file, or None if unavailable.
    ///
    /// This method checks if a thumbnail exists in the cache and is fresh.
    /// It does not trigger generation if the thumbnail is missing.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to get a thumbnail for
    /// * `size` - The desired thumbnail size (e.g., 128 or 256)
    ///
    /// # Returns
    ///
    /// * `Some(PathBuf)` - Path to the cached thumbnail if available and fresh
    /// * `None` - If no thumbnail is available or it's stale
    fn get_thumbnail(&self, entry: &FileEntry, size: u32) -> Option<PathBuf>;

    /// Triggers background generation of a thumbnail for the file.
    ///
    /// This method queues a thumbnail generation task. The thumbnail will
    /// be generated asynchronously and cached for future use.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to generate a thumbnail for
    /// * `size` - The desired thumbnail size (e.g., 128 or 256)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the request was queued successfully
    /// * `Err(ThumbnailError)` - If the file type is unsupported or queuing failed
    fn request_thumbnail(&self, entry: &FileEntry, size: u32) -> Result<(), ThumbnailError>;

    /// Check if the file type is supported for thumbnail generation.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to check
    ///
    /// # Returns
    ///
    /// * `true` - If thumbnails can be generated for this file type
    /// * `false` - If thumbnails are not supported
    fn is_supported(&self, entry: &FileEntry) -> bool;
}

