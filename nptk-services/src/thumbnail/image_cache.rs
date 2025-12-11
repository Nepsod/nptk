//! LRU cache for decoded thumbnail images.
//!
//! This module provides a shared cache for decoded thumbnail images that can
//! be used by any file-related widget (file list, file grid, compact file list, etc.)
//! to avoid re-decoding thumbnails on every render.

use lru::LruCache;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fs;

use crate::filesystem::io_uring;

/// Cached decoded thumbnail image data.
#[derive(Clone)]
pub struct CachedThumbnail {
    /// Raw RGBA pixel data.
    pub data: Arc<Vec<u8>>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// LRU cache for decoded thumbnail images.
///
/// This cache stores decoded PNG thumbnail images in memory to avoid
/// re-decoding them on every render. The cache is keyed by thumbnail
/// path and size.
///
/// This cache is designed to be shared across multiple file-related widgets
/// (file list, file grid, compact file list, etc.) to provide efficient
/// thumbnail rendering.
pub struct ThumbnailImageCache {
    cache: Arc<Mutex<LruCache<(PathBuf, u32), CachedThumbnail>>>,
    max_size: usize,
}

impl ThumbnailImageCache {
    /// Create a new thumbnail image cache with the given maximum size.
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of thumbnails to cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::unbounded())),
            max_size,
        }
    }

    /// Get a cached thumbnail image.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the thumbnail file
    /// * `size` - Thumbnail size (used as part of cache key)
    ///
    /// # Returns
    ///
    /// * `Some(CachedThumbnail)` - If the thumbnail is cached
    /// * `None` - If the thumbnail is not in cache
    pub fn get(&self, path: &PathBuf, size: u32) -> Option<CachedThumbnail> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(&(path.clone(), size)).cloned()
    }

    /// Put a thumbnail image into the cache.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the thumbnail file
    /// * `size` - Thumbnail size
    /// * `thumbnail` - The decoded thumbnail image
    pub fn put(&self, path: PathBuf, size: u32, thumbnail: CachedThumbnail) {
        let mut cache = self.cache.lock().unwrap();
        cache.put((path, size), thumbnail);
    }

    /// Load and cache a thumbnail from a file path.
    ///
    /// If the thumbnail is already cached, returns the cached version.
    /// Otherwise, loads the PNG file, decodes it, and caches it.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the thumbnail PNG file
    /// * `size` - Thumbnail size
    ///
    /// # Returns
    ///
    /// * `Ok(Some(CachedThumbnail))` - If the thumbnail was loaded successfully
    /// * `Ok(None)` - If the file doesn't exist or couldn't be read
    /// * `Err` - If decoding failed
    pub fn load_or_get(
        &self,
        path: &PathBuf,
        size: u32,
    ) -> Result<Option<CachedThumbnail>, image::ImageError> {
        // Check cache first
        if let Some(cached) = self.get(path, size) {
            return Ok(Some(cached));
        }

        // Load from file; when inside a runtime, avoid blocking it and use std::fs.
        let bytes = if tokio::runtime::Handle::try_current().is_ok() {
            match fs::read(path) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(image::ImageError::IoError(e)),
            }
        } else {
            match tokio::runtime::Handle::try_current()
                .ok()
                .and_then(|handle| handle.block_on(async { io_uring::read(path).await.ok() }))
            {
                Some(bytes) => bytes,
                None => match fs::read(path) {
                    Ok(bytes) => bytes,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(e) => return Err(image::ImageError::IoError(e)),
                },
            }
        };

        let img = image::load_from_memory(&bytes)?;

        // Convert to RGBA
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let data = rgba.into_raw();

        let cached = CachedThumbnail {
            data: Arc::new(data),
            width,
            height,
        };

        // Cache it
        self.put(path.clone(), size, cached.clone());

        Ok(Some(cached))
    }

    /// Clear the cache.
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Get the number of cached thumbnails.
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.is_empty()
    }
}

impl Default for ThumbnailImageCache {
    fn default() -> Self {
        Self::new(100) // Default to 100 thumbnails
    }
}
