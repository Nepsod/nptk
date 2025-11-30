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

    /// Map MIME type to icon name according to freedesktop.org Icon Naming Specification.
    /// 
    /// The specification states that MIME types map to icon names by replacing "/" with "-".
    /// For example: text/plain -> text-plain, text/x-toml -> text-x-toml
    /// However, many themes use simplified names like "text-x-generic" for generic text files.
    fn mime_to_icon_name(mime_type: &str) -> String {
        let (main_type, sub_type) = if let Some((m, s)) = mime_type.split_once('/') {
            (m, s)
        } else {
            return "unknown".to_string();
        };

        match main_type {
            "text" => {
                // Text types: use text-x-{subtype} format
                match sub_type {
                    "plain" => "text-x-generic".to_string(),
                    "x-toml" => "text-x-toml".to_string(),
                    "x-rust" | "rust" => "text-x-rust".to_string(),
                    "x-c" | "c" => "text-x-c".to_string(),
                    "x-c++" | "x-cpp" | "cpp" => "text-x-cpp".to_string(),
                    "x-python" | "python" => "text-x-python".to_string(),
                    "x-java" | "java" => "text-x-java".to_string(),
                    "x-javascript" | "javascript" => "text-x-javascript".to_string(),
                    "x-typescript" | "typescript" => "text-x-typescript".to_string(),
                    "x-markdown" | "markdown" => "text-x-markdown".to_string(),
                    "x-yaml" | "yaml" => "text-x-yaml".to_string(),
                    "x-json" | "json" => "text-x-json".to_string(),
                    "x-css" | "css" => "text-x-css".to_string(),
                    "x-html" | "html" => "text-x-html".to_string(),
                    "x-xml" | "xml" => "text-x-xml".to_string(),
                    "x-shellscript" | "shell" => "text-x-script".to_string(),
                    "x-perl" | "perl" => "text-x-perl".to_string(),
                    "x-ruby" | "ruby" => "text-x-ruby".to_string(),
                    "x-php" | "php" => "text-x-php".to_string(),
                    "x-go" | "go" => "text-x-go".to_string(),
                    _ => {
                        // For other text subtypes, try text-x-{subtype} or fallback to generic
                        if sub_type.starts_with("x-") {
                            format!("text-{}", sub_type)
                        } else {
                            format!("text-x-{}", sub_type)
                        }
                    }
                }
            }
            "image" => {
                match sub_type {
                    "png" => "image-x-generic".to_string(),
                    "jpeg" | "jpg" => "image-x-generic".to_string(),
                    "gif" => "image-x-generic".to_string(),
                    "svg+xml" | "svg" => "image-x-generic".to_string(),
                    _ => "image-x-generic".to_string(),
                }
            }
            "video" => "video-x-generic".to_string(),
            "audio" => "audio-x-generic".to_string(),
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
                    "x-rust" | "rust" => "text-x-rust".to_string(), // Rust source files
                    "octet-stream" => "application-x-executable".to_string(),
                    _ => {
                        // For other application subtypes, try application-x-{subtype}
                        if sub_type.starts_with("x-") {
                            format!("application-{}", sub_type)
                        } else {
                            format!("application-x-{}", sub_type)
                        }
                    }
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

    /// Get MIME type from file extension with improved detection for common types.
    fn mime_type_from_extension(ext: &str) -> Option<String> {
        let ext_lower = ext.to_lowercase();
        
        // Handle common file types that might not be in mime_guess database
        let mime_type = match ext_lower.as_str() {
            "toml" => Some("text/x-toml".to_string()),
            "rs" => Some("text/x-rust".to_string()),
            "go" => Some("text/x-go".to_string()),
            "py" => Some("text/x-python".to_string()),
            "js" => Some("text/javascript".to_string()),
            "ts" => Some("text/x-typescript".to_string()),
            "jsx" => Some("text/javascript".to_string()),
            "tsx" => Some("text/x-typescript".to_string()),
            "md" => Some("text/x-markdown".to_string()),
            "yaml" | "yml" => Some("text/x-yaml".to_string()),
            "sh" => Some("text/x-shellscript".to_string()),
            "bash" => Some("text/x-shellscript".to_string()),
            "zsh" => Some("text/x-shellscript".to_string()),
            "fish" => Some("text/x-shellscript".to_string()),
            "c" => Some("text/x-c".to_string()),
            "cpp" | "cxx" | "cc" => Some("text/x-cpp".to_string()),
            "h" | "hpp" => Some("text/x-cpp".to_string()),
            "java" => Some("text/x-java".to_string()),
            "php" => Some("text/x-php".to_string()),
            "rb" => Some("text/x-ruby".to_string()),
            "pl" => Some("text/x-perl".to_string()),
            "css" => Some("text/css".to_string()),
            "html" | "htm" => Some("text/html".to_string()),
            "xml" => Some("text/xml".to_string()),
            "json" => Some("application/json".to_string()),
            _ => {
                // Fallback to mime_guess
                let guessed = mime_guess::from_ext(&ext_lower).first_or_text_plain();
                Some(guessed.to_string())
            }
        };

        mime_type
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

        // Determine MIME type with improved detection
        let mime_type: Option<String> = if let Some(ref mime) = entry.metadata.mime_type {
            log::debug!("MimeIconProvider: Using MIME type from metadata: {}", mime);
            Some(mime.clone())
        } else if let Some(ext) = entry.extension() {
            // Try to get MIME type from extension, with fallbacks for common types
            log::debug!("MimeIconProvider: Detecting MIME type from extension: {}", ext);
            let detected = Self::mime_type_from_extension(ext);
            if let Some(ref mime) = detected {
                log::debug!("MimeIconProvider: Detected MIME type: {}", mime);
            }
            detected
        } else {
            log::debug!("MimeIconProvider: No extension found for file: {}", entry.name);
            None
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

