use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use xdg::BaseDirectories;
use nptk_theme::config::ThemeConfig;

/// The main configuration structure for the application.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// General application settings
    #[serde(default)]
    pub general: GeneralSettings,
    /// Theme settings
    #[serde(default)]
    pub theme: ThemeSettings,
    /// Any other sections are captured here
    #[serde(flatten)]
    pub other: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeneralSettings {
    pub debug: bool,
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeSettings {
    pub name: Option<String>,
    pub variant: Option<String>,
}

/// Registry for managing application settings.
pub struct SettingsRegistry {
    config: Config,
    pub theme_config: ThemeConfig,
}

impl SettingsRegistry {
    /// Create a new SettingsRegistry and load configuration from standard locations.
    pub fn new() -> Result<Self> {
        let mut registry = Self {
            config: Config::default(),
            theme_config: ThemeConfig::new(),
        };
        registry.load()?;
        Ok(registry)
    }

    /// Load configuration from standard locations in precedence order.
    ///
    /// Order (later overrides earlier):
    /// 1. System Data: /usr/share/nptk-0/config.toml (and XDG_DATA_DIRS)
    /// 2. System Config: /etc/nptk-0/config.toml (and XDG_CONFIG_DIRS)
    /// 3. User Config: ~/.config/nptk-0/config.toml (XDG_CONFIG_HOME)
    pub fn load(&mut self) -> Result<()> {
        let xdg_dirs = BaseDirectories::with_prefix("nptk-0")?;
        let config_filename = "config.toml";

        // 1. Load from system data directories (e.g., /usr/share/nptk-0/config.toml)
        let data_config_paths = xdg_dirs.find_data_files(config_filename);
        // Note: find_data_files returns iterator, order depends on XDG_DATA_DIRS.
        // Usually /usr/local/share comes before /usr/share.
        // We want to load them in reverse order of preference so that more specific ones override.
        // However, typically we just want to load *all* of them.
        // Let's assume standard XDG behavior: earlier in list = higher priority.
        // But for *merging* configs, we usually want base -> override.
        // So we should iterate in reverse if the list is priority-ordered.
        // xdg crate docs say: "The order of the paths corresponds to the order of the directories in XDG_DATA_DIRS."
        // XDG_DATA_DIRS defaults to /usr/local/share/:/usr/share/
        // So /usr/local/share is higher priority.
        // If we want /usr/local/share to override /usr/share, we should load /usr/share first.
        for path in data_config_paths.rev() {
            self.load_file(&path);
        }

        // 2. Load from system config directories (e.g., /etc/nptk-0/config.toml)
        let system_config_paths = xdg_dirs.find_config_files(config_filename);
        // XDG_CONFIG_DIRS defaults to /etc/xdg
        for path in system_config_paths.rev() {
            self.load_file(&path);
        }

        // 3. Load from user config directory (e.g., ~/.config/nptk-0/config.toml)
        if let Some(user_config_path) = xdg_dirs.find_config_file(config_filename) {
            self.load_file(&user_config_path);
        } else {
            // Check if it exists but wasn't found (e.g. if we need to create it, but we are just loading here)
            // If find_config_file returns None, it means it doesn't exist.
            // But we might want to check the standard location just in case.
            let user_config_path = xdg_dirs.get_config_home().join(config_filename);
            if user_config_path.exists() {
                self.load_file(&user_config_path);
            }
        }

        self.load_theme_config(&xdg_dirs);

        Ok(())
    }

    /// Load theme configuration from standard locations.
    fn load_theme_config(&mut self, xdg_dirs: &BaseDirectories) {
        let theme_filename = "theme.toml";

        // 1. Load from system data directories
        for path in xdg_dirs.find_data_files(theme_filename).rev() {
            self.load_theme_file(&path);
        }

        // 2. Load from system config directories
        for path in xdg_dirs.find_config_files(theme_filename).rev() {
            self.load_theme_file(&path);
        }

        // 3. Load from user config directory
        if let Some(user_config_path) = xdg_dirs.find_config_file(theme_filename) {
            self.load_theme_file(&user_config_path);
        } else {
            let user_config_path = xdg_dirs.get_config_home().join(theme_filename);
            if user_config_path.exists() {
                self.load_theme_file(&user_config_path);
            }
        }
    }

    fn load_theme_file(&mut self, path: &Path) {
        log::info!("Loading theme config from: {:?}", path);
        match ThemeConfig::from_file(path) {
            Ok(loaded_config) => {
                self.theme_config.merge(loaded_config);
            }
            Err(e) => {
                log::warn!("Failed to load theme config {:?}: {}", path, e);
            }
        }
    }

    fn load_file(&mut self, path: &Path) {
        log::info!("Loading config from: {:?}", path);
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<Config>(&content) {
                    Ok(loaded_config) => {
                        self.merge(loaded_config);
                    }
                    Err(e) => {
                        log::error!("Failed to parse config file {:?}: {}", path, e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to read config file {:?}: {}", path, e);
            }
        }
    }

    /// Merge a loaded config into the current config.
    fn merge(&mut self, other: Config) {
        // Simple merge for now. In a real app, you might want deep merging for HashMaps.
        // For booleans/options, we can just overwrite if they are "set" (non-default).
        // But since we are using serde defaults, it's hard to know if a value was explicitly set or default.
        // A common strategy is to deserialize into Option<T> fields to detect presence.
        // For this implementation, we'll do a basic field-level override.

        // General
        if other.general.debug { self.config.general.debug = true; }
        if other.general.log_level.is_some() { self.config.general.log_level = other.general.log_level; }

        // Theme
        if other.theme.name.is_some() { self.config.theme.name = other.theme.name; }
        if other.theme.variant.is_some() { self.config.theme.variant = other.theme.variant; }

        // Other
        self.config.other.extend(other.other);
    }

    /// Get the current configuration.
    pub fn get(&self) -> &Config {
        &self.config
    }
}


