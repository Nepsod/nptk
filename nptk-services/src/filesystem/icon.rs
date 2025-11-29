//! Icon provider for filesystem entries.

use crate::filesystem::entry::FileEntry;
use mime_guess;

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

    /// Map MIME type to icon name.
    fn mime_to_icon_name(mime_type: &str) -> String {
        let (main_type, sub_type) = if let Some((m, s)) = mime_type.split_once('/') {
            (m, s)
        } else {
            return "unknown".to_string();
        };

        match main_type {
            "text" => format!("text-x-{}", sub_type.replace('-', "-")),
            "image" => format!("image-x-generic"),
            "video" => format!("video-x-generic"),
            "audio" => format!("audio-x-generic"),
            "application" => {
                match sub_type {
                    "pdf" => "application-pdf".to_string(),
                    "zip" | "x-zip-compressed" => "application-zip".to_string(),
                    "x-tar" => "application-x-tar".to_string(),
                    "x-gzip" => "application-x-gzip".to_string(),
                    "x-bzip2" => "application-x-bzip".to_string(),
                    "x-7z-compressed" => "application-x-7z-compressed".to_string(),
                    "x-rar-compressed" => "application-x-rar".to_string(),
                    "json" => "application-json".to_string(),
                    "xml" => "application-xml".to_string(),
                    "javascript" | "x-javascript" => "application-x-javascript".to_string(),
                    "x-sh" | "x-shellscript" => "application-x-shellscript".to_string(),
                    "x-executable" | "x-sharedlib" => "application-x-executable".to_string(),
                    _ => "application-x-generic".to_string(),
                }
            }
            "inode" => {
                match sub_type {
                    "directory" => "folder".to_string(),
                    "symlink" => "inode-symlink".to_string(),
                    _ => "inode-generic".to_string(),
                }
            }
            _ => format!("{}-x-generic", main_type),
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

        // Use MIME type if available
        if let Some(ref mime_type) = entry.metadata.mime_type {
            let icon_name = Self::mime_to_icon_name(mime_type);
            return Some(IconData {
                name: icon_name,
                path: None,
            });
        }

        // Fallback: guess MIME type from file extension
        if let Some(ext) = entry.extension() {
            let mime_type = mime_guess::from_ext(ext).first_or_text_plain();
            let icon_name = Self::mime_to_icon_name(mime_type.as_ref());
            return Some(IconData {
                name: icon_name,
                path: None,
            });
        }

        // Final fallback: generic file icon
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

