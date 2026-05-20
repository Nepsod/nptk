use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs::{Fs, RealFs};
use gpui::{App, AssetSource, Result, SharedString};
use paths::{config_dir, global_settings_file, settings_file, themes_dir};
use settings::{ParseStatus, SettingsStore};
use theme::{LoadThemes, ThemeRegistry};
use theme_settings;

/// Ensures `~/.config/nptk/` exists and seeds missing config files with defaults.
pub fn ensure_config_directory() -> std::io::Result<()> {
    std::fs::create_dir_all(config_dir())?;
    std::fs::create_dir_all(themes_dir())?;

    ensure_config_file(
        settings_file(),
        settings::initial_user_settings_content().as_ref(),
    )?;
    ensure_config_file(global_settings_file(), "{}")?;

    Ok(())
}

fn ensure_config_file(path: &Path, default_content: &str) -> std::io::Result<()> {
    if path.is_file() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, default_content)?;
    log::info!("created {}", path.display());
    Ok(())
}

/// Loads `settings.json` / `global_settings.json` from the NPTK config directory and
/// watches them for changes.
pub fn init_settings(cx: &mut App) {
    if let Err(error) = ensure_config_directory() {
        log::warn!("failed to create NPTK config directory: {error:#}");
    }

    let fs = Arc::new(RealFs::new(None, cx.background_executor().clone()));
    <dyn Fs>::set_global(fs.clone(), cx);

    settings::init(cx);

    SettingsStore::update(cx, |store, cx| {
        store.watch_settings_files(fs, cx, |settings_file, result, _cx| {
            if matches!(result.parse_status, ParseStatus::Failed { .. }) {
                log::warn!(
                    "failed to load {settings_file:?} settings: {:?}",
                    result.parse_error()
                );
            }
        });
    });
}

/// Loads `.json` theme families from `~/.config/nptk/themes/`.
///
/// Call after [`theme_settings::init`] so the theme registry exists.
pub fn load_user_themes_from_config_dir(cx: &App) {
    let themes_directory = themes_dir();
    let Ok(entries) = std::fs::read_dir(themes_directory) else {
        return;
    };

    let registry = ThemeRegistry::global(cx);
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let Ok(bytes) = std::fs::read(&path) else {
            log::warn!("failed to read theme file at {}", path.display());
            continue;
        };

        if let Err(error) = theme_settings::load_user_theme(&registry, &bytes) {
            log::warn!("failed to load theme from {}: {error:#}", path.display());
        }
    }
}

/// Returns bundled theme assets when `assets/themes/` contains JSON theme files.
pub fn bundled_themes_to_load() -> LoadThemes {
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let themes_directory = assets_root.join("themes");

    if themes_directory.is_dir() && themes_directory_has_json(&themes_directory) {
        LoadThemes::All(Box::new(FilesystemAssetSource::new(assets_root)))
    } else {
        LoadThemes::JustBase
    }
}

fn themes_directory_has_json(directory: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if themes_directory_has_json(&path) {
                return true;
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            return true;
        }
    }

    false
}

struct FilesystemAssetSource {
    root: PathBuf,
}

impl FilesystemAssetSource {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl AssetSource for FilesystemAssetSource {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let full_path = self.root.join(path);
        if !full_path.is_file() {
            return Ok(None);
        }

        let bytes = std::fs::read(full_path)?;
        Ok(Some(Cow::Owned(bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let directory = self.root.join(path);
        let mut paths = Vec::new();
        if directory.is_dir() {
            collect_json_paths(&directory, &self.root, &mut paths)?;
        }
        Ok(paths)
    }
}

fn collect_json_paths(
    directory: &Path,
    root: &Path,
    paths: &mut Vec<SharedString>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_paths(&path, root, paths)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            paths.push(SharedString::from(relative_path.to_string_lossy().replace('\\', "/")));
        }
    }
    Ok(())
}
