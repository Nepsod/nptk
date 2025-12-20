//! Thumbnail generation and caching system.
//!
//! This module provides adapter functions for using npio's ThumbnailService
//! with NPTK's FileEntry types. The actual thumbnail implementation is now
//! provided by npio.

pub mod npio_adapter;

// Re-export adapter functions for convenience
pub use npio_adapter::{file_entry_to_uri, u32_to_thumbnail_size, uri_to_path, thumbnail_size_to_u32};

// Keep error type for backward compatibility (may be used elsewhere)
pub mod error;
pub use error::ThumbnailError;

// Deprecated: Old implementation modules kept for reference but not used
#[deprecated(note = "Use npio::ThumbnailService instead")]
pub mod cache;

#[deprecated(note = "Use npio::ThumbnailService instead")]
pub mod events;

#[deprecated(note = "Use npio::ThumbnailService instead")]
pub mod executor;

#[deprecated(note = "Use npio::ThumbnailImageCache or ThumbnailService::get_thumbnail_image() instead")]
pub mod image_cache;

#[deprecated(note = "Use npio::ThumbnailService instead")]
pub mod thumbnailify_provider;

// Re-export deprecated types for backward compatibility
#[deprecated(note = "Use npio::ThumbnailService instead")]
pub use image_cache::{CachedThumbnail, ThumbnailImageCache};

#[deprecated(note = "Use npio::ThumbnailService instead")]
pub use thumbnailify_provider::ThumbnailifyProvider;

/// Trait for thumbnail providers.
///
/// @deprecated This trait is deprecated. Use npio::ThumbnailService directly.
/// The adapter functions in npio_adapter can help convert between NPTK and npio types.
#[deprecated(note = "Use npio::ThumbnailService instead")]
pub trait ThumbnailProvider: Send + Sync + std::any::Any {
    /// Returns a path to a cached thumbnail for the file, or None if unavailable.
    #[deprecated(note = "Use npio::ThumbnailService::get_thumbnail_path() instead")]
    fn get_thumbnail(&self, entry: &crate::filesystem::entry::FileEntry, size: u32) -> Option<std::path::PathBuf>;

    /// Triggers background generation of a thumbnail for the file.
    #[deprecated(note = "Use npio::ThumbnailService::get_or_generate_thumbnail() instead")]
    fn request_thumbnail(&self, entry: &crate::filesystem::entry::FileEntry, size: u32) -> Result<(), ThumbnailError>;

    /// Check if the file type is supported for thumbnail generation.
    #[deprecated(note = "Use npio::ThumbnailService::is_supported() instead")]
    fn is_supported(&self, entry: &crate::filesystem::entry::FileEntry) -> bool;
}
