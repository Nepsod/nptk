//! Event system for thumbnail generation notifications.

use std::path::PathBuf;
use tokio::sync::broadcast;

/// Events emitted by the thumbnail system.
#[derive(Debug, Clone)]
pub enum ThumbnailEvent {
    /// A thumbnail was successfully generated and cached.
    ThumbnailReady {
        /// Path to the original file.
        entry_path: PathBuf,
        /// Path to the cached thumbnail.
        thumbnail_path: PathBuf,
        /// Thumbnail size.
        size: u32,
    },
    /// Thumbnail generation failed.
    ThumbnailFailed {
        /// Path to the original file.
        entry_path: PathBuf,
        /// Error that occurred.
        error: String,
        /// Thumbnail size.
        size: u32,
    },
}

/// Create a new broadcast channel for thumbnail events.
pub fn create_thumbnail_event_channel() -> broadcast::Sender<ThumbnailEvent> {
    broadcast::channel(100).0 // Buffer up to 100 events
}
