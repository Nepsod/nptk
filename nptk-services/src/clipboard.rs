// SPDX-License-Identifier: LGPL-3.0-only
use copypasta::{ClipboardContext, ClipboardProvider};
use std::path::PathBuf;
use log;

/// A cross-platform clipboard service.
pub struct ClipboardService {
    context: Option<ClipboardContext>,
}

impl ClipboardService {
    /// Create a new clipboard service.
    pub fn new() -> Self {
        let context = match ClipboardContext::new() {
            Ok(ctx) => Some(ctx),
            Err(e) => {
                log::warn!("Failed to initialize clipboard context: {}", e);
                None
            }
        };
        
        Self { context }
    }

    /// Set text to clipboard.
    pub fn set_text(&mut self, text: String) -> Result<(), String> {
        if let Some(ctx) = &mut self.context {
            ctx.set_contents(text).map_err(|e| e.to_string())
        } else {
            Err("Clipboard context not available".to_string())
        }
    }

    /// Get text from clipboard.
    pub fn get_text(&mut self) -> Result<String, String> {
        if let Some(ctx) = &mut self.context {
            ctx.get_contents().map_err(|e| e.to_string())
        } else {
            Err("Clipboard context not available".to_string())
        }
    }

    /// Set files to clipboard using GNOME/KDE compatible format.
    /// 
    /// Formats as:
    /// Line 1: `copy` or `cut`
    /// Subsequent lines: `file:///path/to/file`
    pub fn set_files(&mut self, paths: &[PathBuf], is_cut: bool) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut content = String::with_capacity(1024);
        
        // Header
        content.push_str(if is_cut { "cut\n" } else { "copy\n" });
        
        // Paths
        for path in paths {
            let uri = format!("file://{}", path.display());
            content.push_str(&uri);
            content.push('\n');
        }

        // Remove trailing newline if desired, though Linux clipboard usually keeps it or ignores it. 
        // The format is line-based.

        // Also note: Standard text clipboard is often used for this format on Linux file managers.
        // Some might check specifically for x-special/gnome-copied-files MIME type.
        // copypasta usually sets text/plain. 
        // However, many file managers (Nautilus, Dolphin) will inspect text content to see if it matches this format.
        
        self.set_text(content)
    }

    /// Get files from clipboard if content matches file list format.
    /// 
    /// Returns `Some((paths, is_cut))` if successful.
    pub fn get_files(&mut self) -> Result<Option<(Vec<PathBuf>, bool)>, String> {
        let content = self.get_text()?;
        
        let mut lines = content.lines();
        
        // check first line
        let first_line = match lines.next() {
            Some(l) => l.trim(),
            None => return Ok(None),
        };
        
        let is_cut = match first_line {
            "cut" => true,
            "copy" => false,
            // If it doesn't start with cut/copy, it might be just a list of URIs (text/uri-list)
            // or just random text.
            // Let's try to parse as uri-list if it starts with file://
            val if val.starts_with("file://") => {
                 // reset iterator logic - wait, lines iterator consumes.
                 // We need to re-parse or handle this case.
                 // For now, let's just handle strict gnome format or simple uri list.
                 // If first line wasn't header, we'll treat entire content as potential URI list.
                 // But we need to be careful not to treat random text as files.
                 false // assume copy for simple URI list
            }
            _ => return Ok(None),
        };

        // If we detected header, proceed with rest. If we detected URI, we need to include first line.
        let mut paths = Vec::new();
        
        let process_line = |line: &str| -> Option<PathBuf> {
            let line = line.trim();
            if line.starts_with("file://") {
                // Stripping file:// prefix
                // Note: accurate URI parsing would be better (handling %20 etc), but simple strip is MVP
                // urlencoding::decode would be good if we have it, but for now simple strip.
                // Assuming standard paths without complex escaping for MVP.
                if let Some(path_str) = line.strip_prefix("file://") {
                    // Quick unescape for spaces (%20)
                    let unescaped = path_str.replace("%20", " "); 
                    return Some(PathBuf::from(unescaped));
                }
            }
            None
        };

        if first_line.starts_with("file://") {
             if let Some(p) = process_line(first_line) {
                 paths.push(p);
             }
        }
        
        for line in lines {
             if let Some(p) = process_line(line) {
                 paths.push(p);
             }
        }
        
        if paths.is_empty() {
            return Ok(None);
        }
        
        Ok(Some((paths, is_cut)))
    }
}
