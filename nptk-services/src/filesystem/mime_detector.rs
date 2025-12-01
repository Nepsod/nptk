//! MIME type detection with override table for edge cases.

use std::path::Path;

/// MIME type detector that uses mime_guess2 as primary source with override table.
pub struct MimeDetector;

impl MimeDetector {
    /// Small override table for file types that mime_guess2 doesn't handle well.
    /// Only includes special cases that need manual mapping.
    const MIME_OVERRIDES: &'static [(&'static str, &'static str)] = &[
        ("toml", "text/x-toml"),
        ("rs", "text/x-rust"),
        // Add more overrides only if mime_guess2 doesn't handle them correctly
    ];

    /// Detect MIME type from file path.
    /// 
    /// Uses mime_guess2 as primary source, with override table for edge cases.
    /// Returns None if detection fails.
    pub fn detect_mime_type(path: &Path) -> Option<String> {
        // Check override table first (for edge cases)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            for (override_ext, mime_type) in Self::MIME_OVERRIDES {
                if ext_lower == *override_ext {
                    log::debug!("MimeDetector: Using override for extension '{}': {}", ext, mime_type);
                    return Some(mime_type.to_string());
                }
            }
        }

        // Use mime_guess2 as primary source
        let mime = mime_guess2::from_path(path).first_or_octet_stream();
        let mime_str = mime.to_string();
        
        // Don't return "application/octet-stream" as it's too generic
        if mime_str == "application/octet-stream" {
            log::debug!("MimeDetector: Got generic octet-stream for {:?}, returning None", path);
            None
        } else {
            log::debug!("MimeDetector: Detected MIME type '{}' for {:?}", mime_str, path);
            Some(mime_str)
        }
    }

    /// Detect MIME type from file extension only (when path is not available).
    /// 
    /// This is a fallback for cases where we only have the extension.
    pub fn detect_mime_type_from_ext(ext: &str) -> Option<String> {
        let ext_lower = ext.to_lowercase();
        
        // Check override table first
        for (override_ext, mime_type) in Self::MIME_OVERRIDES {
            if ext_lower == *override_ext {
                log::debug!("MimeDetector: Using override for extension '{}': {}", ext, mime_type);
                return Some(mime_type.to_string());
            }
        }

        // Use mime_guess2
        let mime = mime_guess2::from_ext(&ext_lower).first_or_octet_stream();
        let mime_str = mime.to_string();
        
        // Don't return "application/octet-stream" as it's too generic
        if mime_str == "application/octet-stream" {
            log::debug!("MimeDetector: Got generic octet-stream for extension '{}', returning None", ext);
            None
        } else {
            log::debug!("MimeDetector: Detected MIME type '{}' for extension '{}'", mime_str, ext);
            Some(mime_str)
        }
    }
}

