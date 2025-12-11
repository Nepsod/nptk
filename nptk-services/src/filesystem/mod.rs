//! Filesystem model for nptk, similar to Qt6's QFileSystemModel.
//!
//! Provides async filesystem operations, lazy loading, automatic file watching,
//! caching, and icon support for file manager widgets, file chooser dialogs,
//! and desktop widgets.

pub mod cache;
pub mod entry;
pub mod error;
pub mod icon;
pub mod mime_detector;
pub mod mime_registry;
pub mod model;
pub mod watcher;
pub mod io_uring;

// Re-export public API
pub use entry::{FileEntry, FileMetadata, FileType};
pub use mime_detector::MimeDetector;
pub use mime_registry::MimeRegistry;
pub use model::FileSystemModel;
