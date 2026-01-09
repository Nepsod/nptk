// SPDX-License-Identifier: LGPL-3.0-only
//! File entry and metadata types.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Type of filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
    /// Other type (e.g., device, socket, etc.).
    Other,
}

/// Metadata about a filesystem entry.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Size of the file in bytes.
    pub size: u64,
    /// Last modification time.
    pub modified: SystemTime,
    /// Creation time (if available).
    pub created: Option<SystemTime>,
    /// File permissions (Unix-style).
    pub permissions: u32,
    /// MIME type of the file (if detected).
    pub mime_type: Option<String>,
    /// Whether the file is hidden (starts with '.').
    pub is_hidden: bool,
}

/// A filesystem entry (file or directory).
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path to the entry.
    pub path: PathBuf,
    /// Name of the entry (file or directory name).
    pub name: String,
    /// Type of the entry.
    pub file_type: FileType,
    /// Metadata about the entry.
    pub metadata: FileMetadata,
    /// Parent directory path (if any).
    pub parent: Option<PathBuf>,
}

impl FileEntry {
    /// Create a new file entry.
    pub fn new(
        path: PathBuf,
        name: String,
        file_type: FileType,
        metadata: FileMetadata,
        parent: Option<PathBuf>,
    ) -> Self {
        Self {
            path,
            name,
            file_type,
            metadata,
            parent,
        }
    }

    /// Check if this entry is a file.
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }

    /// Check if this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// Check if this entry is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.file_type == FileType::Symlink
    }

    /// Get the file extension (if any).
    pub fn extension(&self) -> Option<&str> {
        self.path.extension()?.to_str()
    }

    /// Get the parent directory path.
    pub fn parent_path(&self) -> Option<&Path> {
        self.parent.as_deref().or_else(|| self.path.parent())
    }
}
