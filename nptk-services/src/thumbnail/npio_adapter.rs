// SPDX-License-Identifier: LGPL-3.0-only
//! Adapter functions for converting between NPTK types and npio types.

use crate::filesystem::entry::FileEntry;
use std::path::PathBuf;

/// Convert a FileEntry to a file:// URI.
///
/// Uses the same logic as the existing file_uri() function from cache.rs.
///
/// # Arguments
///
/// * `entry` - The file entry to convert
///
/// # Returns
///
/// The file:// URI string
pub fn file_entry_to_uri(entry: &FileEntry) -> String {
    let absolute_path = if entry.path.is_absolute() {
        entry.path.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&entry.path)
    };

    let path_str = absolute_path.to_string_lossy();
    let encoded = urlencoding::encode(&path_str);
    format!("file://{}", encoded)
}

/// Convert a file:// URI back to a PathBuf.
///
/// # Arguments
///
/// * `uri` - The file:// URI string
///
/// # Returns
///
/// * `Some(PathBuf)` - If the URI is valid and can be converted
/// * `None` - If the URI is invalid or not a file:// URI
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    if !uri.starts_with("file://") {
        return None;
    }

    let path_part = uri.trim_start_matches("file://");
    let decoded = urlencoding::decode(path_part).ok()?;
    Some(PathBuf::from(decoded.as_ref()))
}

/// Convert a u32 size to the nearest ThumbnailSize enum value.
///
/// Maps sizes to the closest ThumbnailSize variant:
/// - 0-128 → Normal (128x128)
/// - 129-256 → Large (256x256)
/// - 257-512 → XLarge (512x512)
/// - 513+ → XXLarge (1024x1024)
///
/// # Arguments
///
/// * `size` - The desired thumbnail size in pixels
///
/// # Returns
///
/// The corresponding ThumbnailSize enum value
pub fn u32_to_thumbnail_size(size: u32) -> npio::ThumbnailSize {
    match size {
        0..=128 => npio::ThumbnailSize::Normal,
        129..=256 => npio::ThumbnailSize::Large,
        257..=512 => npio::ThumbnailSize::XLarge,
        _ => npio::ThumbnailSize::XXLarge,
    }
}

/// Get the dimension value for a ThumbnailSize enum.
///
/// Returns the width/height dimension for the given size.
///
/// # Arguments
///
/// * `size` - The ThumbnailSize enum value
///
/// # Returns
///
/// The dimension (width or height, they're the same for thumbnails)
pub fn thumbnail_size_to_u32(size: npio::ThumbnailSize) -> u32 {
    size.dimensions().0
}
