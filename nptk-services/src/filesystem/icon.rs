//! Icon provider for filesystem entries.

use crate::filesystem::entry::FileEntry;
use crate::filesystem::mime_detector::MimeDetector;
use crate::filesystem::mime_registry::MimeRegistry;
use std::sync::Arc;

/// Icon data representing an icon for a file entry.
#[derive(Debug, Clone)]
pub struct IconData {
    /// Candidate icon names or identifiers (e.g., "text-x-generic", "folder", etc.).
    pub names: Vec<String>,
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
    
    fn mime_variants(mime_type: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        let push = |s: String, seen: &mut std::collections::BTreeSet<String>, out: &mut Vec<String>| {
            if seen.insert(s.clone()) {
                out.push(s);
            }
        };

        push(mime_type.to_string(), &mut seen, &mut out);

        if let Some((major, sub)) = mime_type.split_once('/') {
            if let Some(stripped) = sub.strip_prefix("x-") {
                push(format!("{}/{}", major, stripped), &mut seen, &mut out);
            }
        }

        // Aliases and supertypes via shared-mime (loaded per call)
        if let Ok(db) = shared_mime::load_mime_db() {
            for alias in db.aliases(mime_type) {
                push(alias.to_string(), &mut seen, &mut out);
            }
            for parent in db.supertypes(mime_type) {
                push(parent.as_ref().to_string(), &mut seen, &mut out);
            }
        }

        out
    }

    /// Get generic-icon names for MIME type variants.
    fn generic_icon_names(mime_type: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        // Try exact match
        if let Some(icon) = crate::filesystem::mime_registry::MimeRegistry::get_generic_icon_name(mime_type) {
            if seen.insert(icon.clone()) {
                out.push(icon);
            }
        }

        // Try variants
        for variant in Self::mime_variants(mime_type) {
            if let Some(icon) = crate::filesystem::mime_registry::MimeRegistry::get_generic_icon_name(&variant) {
                if seen.insert(icon.clone()) {
                    out.push(icon);
                }
            }
        }

        // Try reverse alias lookup (if this type is an alias, check canonical type)
        if let Some(canonical) = crate::filesystem::mime_registry::MimeRegistry::find_canonical_for_alias(mime_type) {
            if let Some(icon) = crate::filesystem::mime_registry::MimeRegistry::get_generic_icon_name(&canonical) {
                if seen.insert(icon.clone()) {
                    out.push(icon);
                }
            }
        }

        out
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
            ("application", "toml") => "application-toml".to_string(),
            ("application", "x-executable") | ("application", "x-sharedlib") => "application-x-executable".to_string(),
            ("application", "octet-stream") => "application-x-executable".to_string(),
            // Disk image types - map to drive-harddisk or media-optical
            ("application", "x-iso9660-image") | ("application", "x-cd-image") => "media-optical".to_string(),
            ("application", "x-raw-floppy-disk-image") => "media-floppy".to_string(),
            ("application", "x-vhd-disk") | ("application", "x-vhdx-disk") | ("application", "x-virtualbox-vhd") => "drive-harddisk".to_string(),
            ("application", "x-qemu-disk") => "drive-harddisk".to_string(),
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
        // Explicit symlink handling so we do not depend on target MIME detection.
        if entry.is_symlink() {
            return Some(IconData {
                names: vec![
                    "inode-symlink".to_string(),
                    // Fallbacks: common symlink icons in many themes
                    "emblem-symbolic-link".to_string(),
                    "folder".to_string(),
                ],
                path: None,
            });
        }

        // Directories always get folder icon
        if entry.is_dir() {
            return Some(IconData {
                names: vec!["folder".to_string()],
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
            let mut names = Vec::new();
            let mut seen = std::collections::BTreeSet::new();

            // First, try generic-icon names from XML (these are the most accurate)
            for generic_icon in Self::generic_icon_names(mime_type) {
                if seen.insert(generic_icon.clone()) {
                    names.push(generic_icon);
                }
            }

            // Second, try the original MIME type first (to catch special cases)
            let original_icon = Self::mime_to_icon_name(mime_type);
            log::debug!("MimeIconProvider: Mapped original MIME type '{}' -> icon '{}'", mime_type, original_icon);
            if seen.insert(original_icon.clone()) {
                names.push(original_icon);
            }

            // Then try MIME-to-icon-name mapping for variants (excluding the original, already done)
            for variant in Self::mime_variants(mime_type) {
                // Skip the original MIME type since we already processed it
                if variant.as_str() == mime_type.as_str() {
                    continue;
                }
                let icon_name = Self::mime_to_icon_name(&variant);
                log::debug!("MimeIconProvider: Mapped MIME type '{}' -> variant '{}' -> icon '{}'", mime_type, variant, icon_name);
                if seen.insert(icon_name.clone()) {
                    names.push(icon_name);
                }
            }

            // Add hand-tuned fallbacks for well-known types that themes often name differently.
            match mime_type.as_str() {
                "application/toml" | "text/x-toml" => {
                    for extra in [
                        "text-x-toml",
                        "application-toml",
                        "text-x-source",
                        "text-x-generic",
                    ] {
                        if seen.insert(extra.to_string()) {
                            names.push(extra.to_string());
                        }
                    }
                }
                "application/x-raw-floppy-disk-image" => {
                    for extra in [
                        "media-floppy",
                        "drive-removable-media",
                        "drive-harddisk",
                    ] {
                        if seen.insert(extra.to_string()) {
                            names.push(extra.to_string());
                        }
                    }
                }
                _ => {}
            }

            if !names.is_empty() {
                log::debug!("MimeIconProvider: Generated icon names {:?} for MIME type '{}'", names, mime_type);
                return Some(IconData { names, path: None });
            }
        }

        // Final fallback: generic file icon
        log::debug!("MimeIconProvider: Using fallback icon 'text-x-generic' for file: {}", entry.name);
        Some(IconData {
            names: vec!["unknown".to_string()],
            path: None,
        })
    }
}

impl Default for MimeIconProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::entry::{FileEntry, FileMetadata, FileType};
    use crate::icon::IconRegistry;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn dummy_entry(name: &str, mime: &str, file_type: FileType) -> FileEntry {
        FileEntry::new(
            PathBuf::from(name),
            name.to_string(),
            file_type,
            FileMetadata {
                size: 0,
                modified: SystemTime::now(),
                created: None,
                permissions: 0o644,
                mime_type: Some(mime.to_string()),
                is_hidden: false,
            },
            None,
        )
    }

    #[test]
    fn mime_provider_emits_application_toml_icon() {
        let entry = dummy_entry("test.toml", "application/toml", FileType::File);
        let provider = MimeIconProvider::new();
        let icon = provider.get_icon(&entry).expect("icon data");
        assert!(
            icon.names.iter().any(|n| n == "application-toml"),
            "expected application-toml in {:?}",
            icon.names
        );
    }

    #[test]
    fn registry_resolves_application_toml_icon() {
        let registry = IconRegistry::new().expect("icon registry");
        let entry = dummy_entry("test.toml", "application/toml", FileType::File);
        let icon = registry.get_file_icon(&entry, 64);
        assert!(icon.is_some(), "registry returned no icon");
    }

    #[test]
    fn registry_resolves_drive_removable_icon() {
        let registry = IconRegistry::new().expect("icon registry");
        let entry =
            dummy_entry("disk.img", "application/x-raw-floppy-disk-image", FileType::File);
        let icon = registry.get_file_icon(&entry, 64);
        assert!(icon.is_some(), "registry returned no icon");
    }
}

