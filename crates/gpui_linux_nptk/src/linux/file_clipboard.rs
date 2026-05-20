use std::path::{Path, PathBuf};

use gpui::{ClipboardEntry, ClipboardItem, ClipboardString, ExternalPaths};
use url::Url;

pub const FILE_LIST_MIME_TYPE: &str = "text/uri-list";
pub const GNOME_COPIED_FILES_MIME_TYPE: &str = "x-special/gnome-copied-files";

pub fn path_to_file_uri(path: &Path) -> String {
    let absolute = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    Url::from_file_path(&absolute)
        .map(|url| url.to_string())
        .unwrap_or_else(|_| format!("file://{}", absolute.to_string_lossy()))
}

pub fn paths_to_uri_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path_to_file_uri(path))
        .collect::<Vec<_>>()
        .join("\r\n")
}

pub fn paths_to_gnome_copied_files(paths: &[PathBuf], is_cut: bool) -> String {
    let mut lines = vec![if is_cut { "cut" } else { "copy" }.to_string()];
    lines.extend(paths.iter().map(|path| path_to_file_uri(path)));
    lines.join("\n")
}

pub fn clipboard_item_for_paths(paths: Vec<PathBuf>, is_cut: bool) -> ClipboardItem {
    let uri_list = paths_to_uri_list(&paths);
    let gnome_payload = paths_to_gnome_copied_files(&paths, is_cut);
    let metadata = if is_cut { "cut" } else { "copy" }.to_string();

    ClipboardItem {
        entries: vec![
            ClipboardEntry::ExternalPaths(ExternalPaths(paths.into())),
            ClipboardEntry::String(ClipboardString {
                text: uri_list,
                metadata: Some(metadata),
            }),
            ClipboardEntry::String(ClipboardString::new(gnome_payload)),
        ],
    }
}

pub fn uri_list_bytes(item: &ClipboardItem) -> Option<Vec<u8>> {
    if let Some(text) = uri_list_text(item) {
        return Some(text.into_bytes());
    }
    None
}

pub fn gnome_copied_files_bytes(item: &ClipboardItem) -> Option<Vec<u8>> {
    item.entries().iter().find_map(|entry| {
        if let ClipboardEntry::String(clipboard_string) = entry {
            let text = &clipboard_string.text;
            if text.starts_with("copy\n") || text.starts_with("cut\n") {
                return Some(text.as_bytes().to_vec());
            }
        }
        None
    })
}

pub fn paths_from_clipboard_item(item: &ClipboardItem) -> Option<(Vec<PathBuf>, bool)> {
    for entry in item.entries() {
        if let ClipboardEntry::String(clipboard_string) = entry {
            let text = clipboard_string.text.trim();
            if text.starts_with("copy\n") || text.starts_with("cut\n") {
                let is_cut = text.starts_with("cut\n");
                let paths = text
                    .lines()
                    .skip(1)
                    .filter_map(parse_uri_line)
                    .collect::<Vec<_>>();
                if !paths.is_empty() {
                    return Some((paths, is_cut));
                }
            }
            if text.contains("file://") {
                let paths = text
                    .lines()
                    .filter_map(parse_uri_line)
                    .collect::<Vec<_>>();
                if !paths.is_empty() {
                    let is_cut = clipboard_string
                        .metadata
                        .as_deref()
                        .is_some_and(|metadata| metadata == "cut");
                    return Some((paths, is_cut));
                }
            }
        }
        if let ClipboardEntry::ExternalPaths(paths) = entry {
            if !paths.0.is_empty() {
                let is_cut = item.entries().iter().any(|entry| {
                    matches!(
                        entry,
                        ClipboardEntry::String(clipboard_string)
                            if clipboard_string.metadata.as_deref() == Some("cut")
                    )
                });
                return Some((paths.0.to_vec(), is_cut));
            }
        }
    }
    None
}

fn uri_list_text(item: &ClipboardItem) -> Option<String> {
    item.entries().iter().find_map(|entry| match entry {
        ClipboardEntry::String(clipboard_string)
            if clipboard_string.text.contains("file://")
                && !clipboard_string.text.starts_with("copy\n")
                && !clipboard_string.text.starts_with("cut\n") =>
        {
            Some(clipboard_string.text.clone())
        }
        _ => None,
    }).or_else(|| {
        let paths: Vec<PathBuf> = item
            .entries()
            .iter()
            .filter_map(|entry| match entry {
                ClipboardEntry::ExternalPaths(paths) => Some(paths.0.to_vec()),
                _ => None,
            })
            .next()?;
        if paths.is_empty() {
            None
        } else {
            Some(paths_to_uri_list(&paths))
        }
    })
}

fn parse_uri_line(line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    if let Ok(url) = Url::parse(trimmed) {
        return url.to_file_path().ok();
    }
    Some(PathBuf::from(trimmed))
}
