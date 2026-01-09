// SPDX-License-Identifier: LGPL-3.0-only
//! Filesystem model for nptk, similar to Qt6's QFileSystemModel.
//!
//! Provides async filesystem operations, lazy loading, automatic file watching,
//! caching, and icon support for file manager widgets, file chooser dialogs,
//! and desktop widgets.

pub mod cache;
pub mod entry;
pub mod error;
pub mod model;

// Re-export public API
pub use entry::{FileEntry, FileMetadata, FileType};
pub use model::FileSystemModel;
