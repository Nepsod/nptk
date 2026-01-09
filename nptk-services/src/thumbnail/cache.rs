// SPDX-License-Identifier: LGPL-3.0-only
//! Cache path computation and freshness checks for thumbnails.
//!
//! This module implements freedesktop.org Thumbnail Managing Standard
//! for cache paths and naming conventions.

use crate::filesystem::entry::FileEntry;
use std::fs;
use std::path::{Path, PathBuf};

/// Compute the cache directory path for thumbnails of a given size.
///
/// Follows freedesktop.org standard: `~/.cache/nptk/thumbnails/{size}/`
///
/// # Arguments
///
/// * `size` - The thumbnail size (e.g., 128 or 256)
///
/// # Returns
///
/// Path to the cache directory for the given size
pub fn thumbnail_cache_dir(size: u32) -> PathBuf {
    let cache_base = dirs::cache_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
    });

    cache_base
        .join("nptk")
        .join("thumbnails")
        .join(size.to_string())
}

/// Compute the cache path for a thumbnail of a file entry.
///
/// The filename is computed as: `{md5(file_uri)}.png`
/// where the URI is `file://` + absolute path (percent-encoded).
///
/// # Arguments
///
/// * `entry` - The file entry
/// * `size` - The thumbnail size (e.g., 128 or 256)
///
/// # Returns
///
/// Path to the cached thumbnail file
pub fn thumbnail_cache_path(entry: &FileEntry, size: u32) -> PathBuf {
    let cache_dir = thumbnail_cache_dir(size);
    let uri = file_uri(&entry.path);
    let md5_hash = file_uri_to_md5(&uri);
    cache_dir.join(format!("{}.png", md5_hash))
}

/// Convert a file path to a file:// URI.
///
/// The path is converted to an absolute path and percent-encoded.
///
/// # Arguments
///
/// * `path` - The file path
///
/// # Returns
///
/// The file:// URI string
pub fn file_uri(path: &Path) -> String {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    let path_str = absolute_path.to_string_lossy();
    let encoded = urlencoding::encode(&path_str);
    format!("file://{}", encoded)
}

/// Compute MD5 hash of a file URI.
///
/// This is used to generate the thumbnail filename according to
/// freedesktop.org Thumbnail Managing Standard.
///
/// # Arguments
///
/// * `uri` - The file URI
///
/// # Returns
///
/// MD5 hash as a hexadecimal string
pub fn file_uri_to_md5(uri: &str) -> String {
    let digest = md5::compute(uri.as_bytes());
    format!("{:x}", digest)
}

/// Check if a thumbnail is fresh (not stale).
///
/// A thumbnail is considered fresh if:
/// 1. The thumbnail file exists
/// 2. The thumbnail's modification time is newer than or equal to the file's modification time
///
/// # Arguments
///
/// * `thumbnail_path` - Path to the thumbnail file
/// * `file_path` - Path to the original file
///
/// # Returns
///
/// * `true` - If the thumbnail exists and is fresh
/// * `false` - If the thumbnail doesn't exist or is stale
pub fn is_thumbnail_fresh(thumbnail_path: &Path, file_path: &Path) -> bool {
    // Check if thumbnail exists
    let thumbnail_metadata = match fs::metadata(thumbnail_path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    // Check if file exists
    let file_metadata = match fs::metadata(file_path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    // Get modification times
    let thumbnail_mtime = match thumbnail_metadata.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };

    let file_mtime = match file_metadata.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Thumbnail is fresh if it's newer than or equal to the file
    thumbnail_mtime >= file_mtime
}

/// Ensure the thumbnail cache directory exists.
///
/// Creates the cache directory and any necessary parent directories.
///
/// # Arguments
///
/// * `size` - The thumbnail size
///
/// # Returns
///
/// * `Ok(())` - If the directory was created or already exists
/// * `Err(std::io::Error)` - If directory creation failed
pub fn ensure_cache_dir(size: u32) -> std::io::Result<()> {
    let cache_dir = thumbnail_cache_dir(size);
    fs::create_dir_all(&cache_dir)?;
    log::debug!("Thumbnail cache directory ensured: {:?}", cache_dir);
    Ok(())
}
