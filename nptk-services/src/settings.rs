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
    /// Mouse settings (input config)
    #[serde(default)]
    pub mouse: MouseSettings,
    /// Keyboard settings (input config)
    #[serde(default)]
    pub keyboard: KeyboardSettings,
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
pub struct MouseSettings {
    pub natural_scrolling: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KeyboardSettings {
    // Placeholder for future keyboard settings
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
        
        // Load config.toml
        self.load_config_type(&xdg_dirs, "config.toml");
        
        // Load input.toml
        self.load_config_type(&xdg_dirs, "input.toml");

        self.load_theme_config(&xdg_dirs);

        Ok(())
    }

    fn load_config_type(&mut self, xdg_dirs: &BaseDirectories, filename: &str) {
        // 1. Load from system data directories
        for path in xdg_dirs.find_data_files(filename).rev() {
            self.load_file(&path);
        }

        // 2. Load from system config directories
        for path in xdg_dirs.find_config_files(filename).rev() {
            self.load_file(&path);
        }

        // 3. Load from user config directory
        if let Some(user_config_path) = xdg_dirs.find_config_file(filename) {
            self.load_file(&user_config_path);
        } else {
            let user_config_path = xdg_dirs.get_config_home().join(filename);
            if user_config_path.exists() {
                self.load_file(&user_config_path);
            }
        }
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
        // General
        if other.general.debug { self.config.general.debug = true; }
        if other.general.log_level.is_some() { self.config.general.log_level = other.general.log_level; }

        // Mouse
        if other.mouse.natural_scrolling { self.config.mouse.natural_scrolling = true; }
        
        // Keyboard - nothing to merge yet

        // Other
        self.config.other.extend(other.other);
    }

    /// Get the current configuration.
    pub fn get(&self) -> &Config {
        &self.config
    }
}


