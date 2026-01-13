// SPDX-License-Identifier: LGPL-3.0-only
use anyhow::Result;
use nptk_theme::config::ThemeConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use xdg::BaseDirectories;
use smol::fs;

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
    pub debug: Option<bool>,
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MouseSettings {
    pub natural_scrolling: Option<bool>,
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
    pub async fn new() -> Result<Self> {
        let mut registry = Self {
            config: Config {
                general: GeneralSettings {
                    debug: Some(false),
                    log_level: None,
                },
                mouse: MouseSettings {
                    natural_scrolling: Some(false),
                },
                keyboard: KeyboardSettings {},
                other: HashMap::new(),
            },
            theme_config: ThemeConfig::new(),
        };
        registry.load().await?;
        Ok(registry)
    }

    /// Load configuration from standard locations in precedence order.
    ///
    /// Order (later overrides earlier):
    /// 1. System Data: /usr/share/nptk-0/config.toml (and XDG_DATA_DIRS)
    /// 2. System Config: /etc/nptk-0/config.toml (and XDG_CONFIG_DIRS)
    /// 3. User Config: ~/.config/nptk-0/config.toml (XDG_CONFIG_HOME)
    pub async fn load(&mut self) -> Result<()> {
        let xdg_dirs = BaseDirectories::with_prefix("nptk-0")?;

        // Load config.toml
        self.load_config_type(&xdg_dirs, "config.toml").await;

        // Load input.toml
        self.load_config_type(&xdg_dirs, "input.toml").await;

        self.load_theme_config(&xdg_dirs).await;

        Ok(())
    }

    async fn load_config_type(&mut self, xdg_dirs: &BaseDirectories, filename: &str) {
        // 1. Load from system data directories
        for path in xdg_dirs.find_data_files(filename).rev() {
            self.load_file(&path).await;
        }

        // 2. Load from system config directories
        for path in xdg_dirs.find_config_files(filename).rev() {
            self.load_file(&path).await;
        }

        // 3. Load from user config directory
        if let Some(user_config_path) = xdg_dirs.find_config_file(filename) {
            self.load_file(&user_config_path).await;
        } else {
            let user_config_path = xdg_dirs.get_config_home().join(filename);
            if user_config_path.exists() {
                self.load_file(&user_config_path).await;
            }
        }
    }

    /// Load theme configuration from standard locations.
    async fn load_theme_config(&mut self, xdg_dirs: &BaseDirectories) {
        let theme_filename = "theme.toml";

        // 1. Load from system data directories
        for path in xdg_dirs.find_data_files(theme_filename).rev() {
            self.load_theme_file(&path).await;
        }

        // 2. Load from system config directories
        for path in xdg_dirs.find_config_files(theme_filename).rev() {
            self.load_theme_file(&path).await;
        }

        // 3. Load from user config directory
        if let Some(user_config_path) = xdg_dirs.find_config_file(theme_filename) {
            self.load_theme_file(&user_config_path).await;
        } else {
            let user_config_path = xdg_dirs.get_config_home().join(theme_filename);
            if user_config_path.exists() {
                self.load_theme_file(&user_config_path).await;
            }
        }
    }

    async fn load_theme_file(&mut self, path: &Path) {
        log::info!("Loading theme config from: {:?}", path);
        match fs::read_to_string(path).await {
            Ok(content) => match ThemeConfig::from_toml(&content) {
                Ok(loaded_config) => {
                    self.theme_config.merge(loaded_config);
                },
                Err(e) => {
                    log::warn!("Failed to parse theme config {:?}: {}", path, e);
                },
            },
            Err(e) => {
                log::warn!("Failed to read theme config {:?}: {}", path, e);
            },
        }
    }

    async fn load_file(&mut self, path: &Path) {
        log::info!("Loading config from: {:?}", path);
        match fs::read_to_string(path).await {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(loaded_config) => {
                    self.merge(loaded_config);
                },
                Err(e) => {
                    log::error!("Failed to parse config file {:?}: {}", path, e);
                },
            },
            Err(e) => {
                log::warn!("Failed to read config file {:?}: {}", path, e);
            },
        }
    }

    /// Merge a loaded config into the current config.
    fn merge(&mut self, other: Config) {
        // General
        if let Some(debug) = other.general.debug {
            self.config.general.debug = Some(debug);
        }
        if other.general.log_level.is_some() {
            self.config.general.log_level = other.general.log_level;
        }

        // Mouse
        if let Some(natural) = other.mouse.natural_scrolling {
            self.config.mouse.natural_scrolling = Some(natural);
        }

        // Keyboard - nothing to merge yet

        // Other
        self.config.other.extend(other.other);
    }

    /// Get the current configuration.
    pub fn get(&self) -> &Config {
        &self.config
    }

    /// Load configuration from multiple custom paths asynchronously.
    pub async fn load_from_paths_async(&mut self, paths: Vec<std::path::PathBuf>) -> Vec<anyhow::Result<()>> {
        let mut results = Vec::new();
        
        for path in paths {
            let result = async {
                let content = smol::fs::read_to_string(&path).await
                    .map_err(|e| anyhow::anyhow!("Failed to read config file {:?}: {}", path, e))?;
                
                let loaded_config: Config = toml::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse config file {:?}: {}", path, e))?;
                
                self.merge(loaded_config);
                Ok(())
            }.await;
            
            results.push(result);
        }
        
        results
    }

    /// Reload configuration asynchronously (re-runs the full load process).
    pub async fn reload_async(&mut self) -> anyhow::Result<()> {
        // Reset to defaults
        *self = Self {
            config: Config {
                general: GeneralSettings {
                    debug: Some(false),
                    log_level: None,
                },
                mouse: MouseSettings {
                    natural_scrolling: Some(false),
                },
                keyboard: KeyboardSettings {},
                other: HashMap::new(),
            },
            theme_config: nptk_theme::config::ThemeConfig::new(),
        };
        
        // Reload everything
        self.load().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_boolean_override() {
        let mut registry = SettingsRegistry {
            config: Config {
                general: GeneralSettings {
                    debug: Some(true),
                    log_level: None,
                },
                mouse: MouseSettings {
                    natural_scrolling: Some(true),
                },
                keyboard: KeyboardSettings {},
                other: HashMap::new(),
            },
            theme_config: ThemeConfig::new(),
        };

        let new_config = Config {
            general: GeneralSettings {
                debug: Some(false), // Should override to false
                log_level: None,
            },
            mouse: MouseSettings {
                natural_scrolling: Some(false), // Should override to false
            },
            keyboard: KeyboardSettings {},
            other: HashMap::new(),
        };

        registry.merge(new_config);

        // This is expected to FAIL currently because merge logic only enables, doesn't disable
        assert_eq!(registry.config.general.debug, Some(false), "Debug should be false");
        assert_eq!(registry.config.mouse.natural_scrolling, Some(false), "Natural scrolling should be false");
    }
}
