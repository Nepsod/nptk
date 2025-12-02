//! Icon provider for filesystem entries.

use crate::filesystem::entry::FileEntry;
use crate::filesystem::mime_detector::MimeDetector;

/// Icon data representing an icon for a file entry.
#[derive(Debug, Clone)]
pub struct IconData {
    /// Icon name or identifier (e.g., "text-x-generic", "folder", etc.).
    pub name: String,
    /// Optional path to icon file (for system icons).
    pub path: Option<std::path::PathBuf>,
}

/// Trait for providing icons for filesystem entries.
pub trait IconProvider: Send + Sync {
    /// Get icon data for a file entry.
    fn get_icon(&self, entry: &FileEntry) -> Option<IconData>;
}

/// Icon provider based on MIME type detection.
pub struct MimeIconProvider;

impl MimeIconProvider {
    /// Create a new MIME-based icon provider.
    pub fn new() -> Self {
        Self
    }

    /// Map MIME type to icon name according to freedesktop.org Icon Naming Specification.
    /// 
    /// The specification states that MIME types map to icon names by replacing "/" with "-".
    /// General rule: Replace "/" with "-" in MIME type (e.g., text/plain -> text-plain).
    /// However, many themes use simplified names, so we apply some special cases.
    fn mime_to_icon_name(mime_type: &str) -> String {
        let (main_type, sub_type) = if let Some((m, s)) = mime_type.split_once('/') {
            (m, s)
        } else {
            return "unknown".to_string();
        };

        // Special cases that don't follow the simple replacement rule
        match (main_type, sub_type) {
            ("inode", "directory") => "folder".to_string(),
            ("inode", "symlink") => "inode-symlink".to_string(),
            ("text", "plain") => "text-x-generic".to_string(),
            ("application", "pdf") => "application-pdf".to_string(),
            ("application", "zip") | ("application", "x-zip-compressed") => "application-zip".to_string(),
            ("application", "json") => "application-json".to_string(),
            ("application", "xml") => "application-xml".to_string(),
            ("application", "x-executable") | ("application", "x-sharedlib") => "application-x-executable".to_string(),
            ("application", "octet-stream") => "application-x-executable".to_string(),
            _ => {
                // General rule: Replace "/" with "-"
                // For text/x-* types, use text-x-{subtype}
                // For application/x-* types, use application-x-{subtype} or application-{subtype}
                if main_type == "text" {
                    if sub_type.starts_with("x-") {
                        format!("text-{}", sub_type)
                    } else {
                        format!("text-x-{}", sub_type)
                    }
                } else if main_type == "application" {
                    if sub_type.starts_with("x-") {
                        format!("application-{}", sub_type)
                    } else {
                        // Try application-{subtype} first, fallback to application-x-{subtype}
                        format!("application-{}", sub_type)
                }
                } else {
                    // For other types, use the simple replacement rule
                    format!("{}-{}", main_type, sub_type)
                        .replace("+", "-") // Replace + with - (e.g., svg+xml -> svg-xml)
                }
            }
        }
    }
}

impl IconProvider for MimeIconProvider {
    fn get_icon(&self, entry: &FileEntry) -> Option<IconData> {
        // Directories always get folder icon
        if entry.is_dir() {
            return Some(IconData {
                name: "folder".to_string(),
                path: None,
            });
        }

        // Determine MIME type using MimeDetector
        let mime_type: Option<String> = if let Some(ref mime) = entry.metadata.mime_type {
            log::debug!("MimeIconProvider: Using MIME type from metadata: {}", mime);
            Some(mime.clone())
        } else {
            // Use MimeDetector to detect MIME type from path or extension
            if let Some(ext) = entry.extension() {
                log::debug!("MimeIconProvider: Detecting MIME type from extension: {}", ext);
                let detected = MimeDetector::detect_mime_type_from_ext(ext);
                if let Some(ref mime) = detected {
                    log::debug!("MimeIconProvider: Detected MIME type: {}", mime);
                }
                detected
            } else {
                // Try to detect from path (for files without extensions)
                log::debug!("MimeIconProvider: No extension, trying path-based detection for: {}", entry.name);
                MimeDetector::detect_mime_type(&entry.path)
            }
        };

        // Map MIME type to icon name
        if let Some(ref mime_type) = mime_type {
            let icon_name = Self::mime_to_icon_name(mime_type);
            log::debug!("MimeIconProvider: Mapped MIME type '{}' to icon name '{}'", mime_type, icon_name);
            return Some(IconData {
                name: icon_name,
                path: None,
            });
        }

        // Final fallback: generic file icon
        log::debug!("MimeIconProvider: Using fallback icon 'text-x-generic' for file: {}", entry.name);
        Some(IconData {
            name: "text-x-generic".to_string(),
            path: None,
        })
    }
}

impl Default for MimeIconProvider {
    fn default() -> Self {
        Self::new()
    }
}

