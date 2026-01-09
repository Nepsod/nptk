// SPDX-License-Identifier: LGPL-3.0-only
//! In-memory cache for filesystem entries.

use crate::filesystem::entry::FileEntry;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Maximum number of directory entries to cache
const CHILDREN_CACHE_SIZE: usize = 500;
/// Maximum number of file entries to cache
const ENTRIES_CACHE_SIZE: usize = 2000;

/// Thread-safe cache for filesystem entries with LRU eviction.
pub struct FileSystemCache {
    /// LRU cache from directory path to its children entries.
    children: Arc<Mutex<LruCache<PathBuf, Vec<FileEntry>>>>,
    /// LRU cache from file path to its entry (for quick lookup).
    entries: Arc<Mutex<LruCache<PathBuf, FileEntry>>>,
}

impl FileSystemCache {
    /// Create a new empty cache with LRU eviction.
    pub fn new() -> Self {
        Self {
            children: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(CHILDREN_CACHE_SIZE).unwrap()))),
            entries: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(ENTRIES_CACHE_SIZE).unwrap()))),
        }
    }

    /// Get children entries for a directory.
    pub fn get_children(&self, path: &Path) -> Option<Vec<FileEntry>> {
        self.children.lock().unwrap().get(path).cloned()
    }

    /// Insert children entries for a directory.
    pub fn insert_children(&self, path: &Path, entries: Vec<FileEntry>) {
        let mut children = self.children.lock().unwrap();
        let mut entry_map = self.entries.lock().unwrap();

        // Update children cache
        children.put(path.to_path_buf(), entries.clone());

        // Update entry cache for quick lookups
        for entry in entries {
            entry_map.put(entry.path.clone(), entry);
        }
    }

    /// Get a single entry by path.
    pub fn get_entry(&self, path: &Path) -> Option<FileEntry> {
        self.entries.lock().unwrap().get(path).cloned()
    }

    /// Insert or update a single entry.
    pub fn insert_entry(&self, entry: FileEntry) {
        self.entries
            .lock()
            .unwrap()
            .put(entry.path.clone(), entry);
    }

    /// Invalidate (remove) a directory and its children from the cache.
    pub fn invalidate(&self, path: &Path) {
        let mut children = self.children.lock().unwrap();
        let mut entries = self.entries.lock().unwrap();

        // Remove directory from children cache
        if let Some(dir_entries) = children.pop(path) {
            // Remove all child entries from entry cache
            for entry in dir_entries {
                entries.pop(&entry.path);
            }
        }

        // Also remove the directory entry itself if it exists
        entries.pop(path);
    }

    /// Remove a specific entry from the cache.
    pub fn remove_entry(&self, path: &Path) {
        self.entries.lock().unwrap().pop(path);

        // Also remove from parent's children list if we can find it
        if let Some(parent) = path.parent() {
            if let Some(children) = self.children.lock().unwrap().get_mut(parent) {
                children.retain(|e| e.path != path);
            }
        }
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        self.children.lock().unwrap().clear();
        self.entries.lock().unwrap().clear();
    }
}

impl Default for FileSystemCache {
    fn default() -> Self {
        Self::new()
    }
}
