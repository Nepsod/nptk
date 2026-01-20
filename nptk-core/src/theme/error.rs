// SPDX-License-Identifier: LGPL-3.0-only

//! Theme loading and parsing errors.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading or parsing themes.
#[derive(Debug, Error)]
pub enum ThemeError {
    /// Theme file not found.
    #[error("Theme file not found: {0}")]
    NotFound(PathBuf),

    /// Failed to read theme file.
    #[error("Failed to read theme file {0}: {1}")]
    ReadError(PathBuf, std::io::Error),

    /// Failed to parse TOML theme file.
    #[error("Failed to parse theme file {0}: {1}")]
    ParseError(PathBuf, String),

    /// Missing required color role.
    #[error("Missing required color role: {0}")]
    MissingColorRole(String),

    /// Invalid color format.
    #[error("Invalid color format: {0}")]
    InvalidColor(String),

    /// Invalid alignment value.
    #[error("Invalid alignment value: {0}")]
    InvalidAlignment(String),

    /// Invalid metric value.
    #[error("Invalid metric value: {0}")]
    InvalidMetric(String),

    /// Terminal colors file not found.
    #[error("Terminal colors file not found: {0}")]
    TerminalColorsNotFound(String),

    /// Failed to load terminal colors.
    #[error("Failed to load terminal colors: {0}")]
    TerminalColorsError(String),

    /// Theme directory not found.
    #[error("Theme directory not found: {0}")]
    ThemeDirectoryNotFound(PathBuf),

    /// Invalid theme name.
    #[error("Invalid theme name: {0}")]
    InvalidThemeName(String),
}
