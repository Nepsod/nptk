//! Filesystem model for nptk, similar to Qt6's QFileSystemModel.
//!
//! Provides async filesystem operations, lazy loading, automatic file watching,
//! caching, and icon support for file manager widgets, file chooser dialogs,
//! and desktop widgets.

pub mod entry;
pub mod cache;
pub mod watcher;
pub mod model;
pub mod icon;
pub mod error;
pub mod mime_detector;

// Re-export public API
pub use model::FileSystemModel;
pub use entry::{FileEntry, FileType, FileMetadata};
pub use mime_detector::MimeDetector;

