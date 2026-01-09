// SPDX-License-Identifier: LGPL-3.0-only
//! In-memory cache for filesystem entries.

use crate::filesystem::entry::FileEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Thread-safe cache for filesystem entries.
pub struct FileSystemCache {
    /// Map from directory path to its children entries.
    children: Arc<Mutex<HashMap<PathBuf, Vec<FileEntry>>>>,
    /// Map from file path to its entry (for quick lookup).
    entries: Arc<Mutex<HashMap<PathBuf, FileEntry>>>,
}

impl FileSystemCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
            entries: Arc::new(Mutex::new(HashMap::new())),
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

        // Update children map
        children.insert(path.to_path_buf(), entries.clone());

        // Update entry map for quick lookups
        for entry in entries {
            entry_map.insert(entry.path.clone(), entry);
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
            .insert(entry.path.clone(), entry);
    }

    /// Invalidate (remove) a directory and its children from the cache.
    pub fn invalidate(&self, path: &Path) {
        let mut children = self.children.lock().unwrap();
        let mut entries = self.entries.lock().unwrap();

        // Remove directory from children map
        if let Some(dir_entries) = children.remove(path) {
            // Remove all child entries from entry map
            for entry in dir_entries {
                entries.remove(&entry.path);
            }
        }

        // Also remove the directory entry itself if it exists
        entries.remove(path);
    }

    /// Remove a specific entry from the cache.
    pub fn remove_entry(&self, path: &Path) {
        self.entries.lock().unwrap().remove(path);

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
