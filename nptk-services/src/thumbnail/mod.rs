// SPDX-License-Identifier: LGPL-3.0-only
//! Thumbnail generation and caching system.
//!
//! This module provides adapter functions for using npio's ThumbnailService
//! with NPTK's FileEntry types. The actual thumbnail implementation is now
//! provided by npio.

pub mod npio_adapter;

// Re-export adapter functions for convenience
pub use npio_adapter::{file_entry_to_uri, u32_to_thumbnail_size, uri_to_path, thumbnail_size_to_u32};

// Keep error type for backward compatibility (may be used elsewhere)
pub mod error;
pub use error::ThumbnailError;
