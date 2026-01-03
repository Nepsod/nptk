//! # Theme Error Types
//!
//! This module provides comprehensive error types for the theming system,
//! replacing generic error types with specific, context-rich error messages.

use std::path::PathBuf;
use thiserror::Error;

use crate::id::WidgetId;
use crate::properties::ThemeProperty;

/// Errors that can occur in the theming system.
#[derive(Error, Debug)]
pub enum ThemeError {
    /// Theme with the specified name was not found.
    #[error("Theme '{name}' not found")]
    ThemeNotFound {
        /// The name of the theme that was not found.
        name: String,
    },

    /// Theme configuration file was not found.
    #[error("Theme file not found: {path:?}")]
    ThemeFileNotFound {
        /// The path that was not found.
        path: PathBuf,
    },

    /// Error parsing theme configuration file.
    #[error("Failed to parse theme file {path:?}: {details}")]
    ThemeParseError {
        /// The path of the file that failed to parse.
        path: PathBuf,
        /// Details about the parse error.
        details: String,
    },

    /// Required theme property is missing for a widget.
    #[error("Widget {widget:?} is missing required property {property:?}")]
    ThemePropertyMissing {
        /// The widget ID that is missing the property.
        widget: WidgetId,
        /// The property that is missing.
        property: ThemeProperty,
    },

    /// Error loading a theme.
    #[error("Failed to load theme: {source}")]
    ThemeLoadError {
        /// The underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error reloading a theme (hot reload failure).
    #[error("Failed to reload theme: {source}")]
    ThemeReloadError {
        /// The underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error during theme transition.
    #[error("Theme transition error: {message}")]
    ThemeTransitionError {
        /// Error message describing what went wrong.
        message: String,
    },

    /// Error setting up file watcher for hot reload.
    #[error("Failed to setup file watcher: {source}")]
    FileWatcherError {
        /// The underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error serializing or deserializing theme data.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Result type alias for theme operations.
pub type ThemeResult<T> = Result<T, ThemeError>;

impl ThemeError {
    /// Create a theme not found error.
    pub fn not_found(name: impl Into<String>) -> Self {
        Self::ThemeNotFound { name: name.into() }
    }

    /// Create a theme file not found error.
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Self::ThemeFileNotFound { path: path.into() }
    }

    /// Create a theme parse error.
    pub fn parse_error(path: impl Into<PathBuf>, details: impl Into<String>) -> Self {
        Self::ThemeParseError {
            path: path.into(),
            details: details.into(),
        }
    }

    /// Create a theme property missing error.
    pub fn property_missing(widget: WidgetId, property: ThemeProperty) -> Self {
        Self::ThemePropertyMissing { widget, property }
    }

    /// Create a theme load error from any error type.
    pub fn load_error(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::ThemeLoadError {
            source: Box::new(source),
        }
    }

    /// Create a theme reload error from any error type.
    pub fn reload_error(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::ThemeReloadError {
            source: Box::new(source),
        }
    }

    /// Create a theme transition error.
    pub fn transition_error(message: impl Into<String>) -> Self {
        Self::ThemeTransitionError {
            message: message.into(),
        }
    }

    /// Create a file watcher error from any error type.
    pub fn file_watcher_error(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::FileWatcherError {
            source: Box::new(source),
        }
    }
}
