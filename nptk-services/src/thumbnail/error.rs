// SPDX-License-Identifier: LGPL-3.0-only
//! Error types for thumbnail operations.

use thiserror::Error;

/// Errors that can occur during thumbnail operations.
#[derive(Error, Debug)]
pub enum ThumbnailError {
    /// The file type is not supported for thumbnail generation.
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    /// Thumbnail generation failed.
    #[error("Thumbnail generation failed: {0}")]
    GenerationFailed(String),

    /// Cache operation failed.
    #[error("Cache error: {0}")]
    CacheError(String),

    /// I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}
