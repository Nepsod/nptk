mod config;

pub use gpui;
pub use gpui_platform;
pub use gpui_rsx::rsx;
pub use gpui_tokio;
pub use settings;
pub use theme;
pub use theme_settings;
pub use file_icons;
pub use std;
pub use ui;

use gpui::App;

/// Initializes GPUI services required by NPTK UI crates.
///
/// Loads `~/.config/nptk/settings.json` (and `global_settings.json`), watches them for
/// changes, and applies theme settings. Theme JSON files in `~/.config/nptk/themes/` are
/// registered when present. Bundled themes under `assets/themes/` are used when that
/// directory contains `.json` files.
pub fn init(cx: &mut App) {
    gpui_tokio::init(cx);
    config::init_settings(cx);

    let themes_to_load = config::bundled_themes_to_load();
    theme_settings::init(themes_to_load, cx);
    config::load_user_themes_from_config_dir(cx);
    theme_settings::reload_theme(cx);
}
