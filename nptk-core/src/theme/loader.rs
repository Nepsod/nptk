// SPDX-License-Identifier: LGPL-3.0-only

//! Async theme loader for TOML theme files.
//!
//! This module provides functionality for loading themes from TOML files,
//! including parsing of color roles, alignments, flags, metrics, paths,
//! and terminal color schemes.
//!
//! ## TOML Format
//!
//! Theme files use the following TOML structure:
//!
//! ```toml
//! [Colors]
//! Window = "#161925"
//! WindowText = "#C3C7D1"
//! Button = "#181b28"
//!
//! [Alignments]
//! TitleAlignment = "Center"
//!
//! [Flags]
//! BoldTextAsBright = true
//!
//! [Metrics]
//! BorderThickness = 4
//! TitleHeight = 19
//!
//! [TerminalColors]
//! TerminalColors = "theme"
//! ```

use std::path::{Path, PathBuf};
use super::error::ThemeError;
use super::roles::{
    AlignmentRole, ColorRole, FlagRole, MetricRole, PathRole, TextAlignment, WindowThemeProvider,
};
use super::terminal::{resolve_terminal_colors, TerminalColors};
use super::util::parse_hex_color;
use super::Theme;

/// Theme loader for loading themes from TOML files.
pub struct ThemeLoader;

impl ThemeLoader {
    /// Load a theme from a TOML file.
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Theme, ThemeError> {
        let path = path.as_ref();
        let content = smol::fs::read_to_string(path)
            .await
            .map_err(|e| ThemeError::ReadError(path.to_path_buf(), e))?;

        Self::load_from_toml(&content, path).await
    }

    /// Load a theme from TOML content.
    pub async fn load_from_toml<P: AsRef<Path>>(
        content: &str,
        path: P,
    ) -> Result<Theme, ThemeError> {
        let path = path.as_ref();
        let table: toml::Value = toml::from_str(content)
            .map_err(|e| ThemeError::ParseError(path.to_path_buf(), e.to_string()))?;

        let mut theme = Theme::new();
        let theme_dir = path.parent();

        Self::parse_colors_section(&table, &mut theme)?;
        Self::parse_alignments_section(&table, &mut theme)?;
        Self::parse_flags_section(&table, &mut theme);
        Self::parse_metrics_section(&table, &mut theme);
        Self::parse_paths_section(&table, &mut theme);
        Self::parse_window_section(&table, &mut theme);
        Self::parse_terminal_colors_section(&table, &mut theme, theme_dir).await;

        Ok(theme)
    }

    /// Parse the [Colors] section from TOML.
    fn parse_colors_section(table: &toml::Value, theme: &mut Theme) -> Result<(), ThemeError> {
        if let Some(colors) = table.get("Colors").and_then(|v| v.as_table()) {
            for (key, value) in colors.iter() {
                if let Some(color_str) = value.as_str() {
                    if let Some(role) = ColorRole::from_str(key) {
                        let color = parse_hex_color(color_str)?;
                        theme.set_color(role, color);
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse the [Alignments] section from TOML.
    fn parse_alignments_section(table: &toml::Value, theme: &mut Theme) -> Result<(), ThemeError> {
        if let Some(alignments) = table.get("Alignments").and_then(|v| v.as_table()) {
            for (key, value) in alignments.iter() {
                if let Some(alignment_str) = value.as_str() {
                    if let Some(role) = AlignmentRole::from_str(key) {
                        if let Some(alignment) = TextAlignment::from_str(alignment_str) {
                            theme.set_alignment(role, alignment);
                        } else {
                            return Err(ThemeError::InvalidAlignment(alignment_str.to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse the [Flags] section from TOML.
    fn parse_flags_section(table: &toml::Value, theme: &mut Theme) {
        if let Some(flags) = table.get("Flags").and_then(|v| v.as_table()) {
            for (key, value) in flags.iter() {
                if let Some(flag_value) = value.as_bool() {
                    if let Some(role) = FlagRole::from_str(key) {
                        theme.set_flag(role, flag_value);
                    }
                }
            }
        }
    }

    /// Parse the [Metrics] section from TOML.
    fn parse_metrics_section(table: &toml::Value, theme: &mut Theme) {
        if let Some(metrics) = table.get("Metrics").and_then(|v| v.as_table()) {
            for (key, value) in metrics.iter() {
                if let Some(metric_value) = value.as_integer() {
                    if let Some(role) = MetricRole::from_str(key) {
                        theme.set_metric(role, metric_value as i32);
                    }
                }
            }
        }
    }

    /// Parse the [Paths] section from TOML.
    fn parse_paths_section(table: &toml::Value, theme: &mut Theme) {
        if let Some(paths) = table.get("Paths").and_then(|v| v.as_table()) {
            for (key, value) in paths.iter() {
                if let Some(path_str) = value.as_str() {
                    if let Some(role) = PathRole::from_str(key) {
                        let path = PathBuf::from(path_str);
                        theme.set_path(role, path);
                    }
                }
            }
        }
    }

    /// Parse the [Window] section from TOML.
    fn parse_window_section(table: &toml::Value, theme: &mut Theme) {
        if let Some(window) = table.get("Window").and_then(|v| v.as_table()) {
            if let Some(theme_str) = window.get("WindowTheme").and_then(|v| v.as_str()) {
                if let Some(provider) = WindowThemeProvider::from_str(theme_str) {
                    theme.set_window_theme(provider);
                }
            }
        }
    }

    /// Parse the [TerminalColors] section from TOML.
    async fn parse_terminal_colors_section(
        table: &toml::Value,
        theme: &mut Theme,
        theme_dir: Option<&Path>,
    ) {
        if let Some(term_colors) = table.get("TerminalColors").and_then(|v| v.as_table()) {
            let terminal_colors_value = term_colors
                .get("TerminalColors")
                .and_then(|v| v.as_str())
                .unwrap_or("theme");
            let override_builtin = term_colors
                .get("OverrideBuiltinTermColors")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            match resolve_terminal_colors(terminal_colors_value, theme_dir, override_builtin).await {
                Ok(colors) => {
                    theme.set_terminal_colors(colors);
                }
                Err(e) => {
                    // Log warning but don't fail theme loading
                    log::warn!("Failed to load terminal colors: {}", e);
                }
            }
        }
    }

    /// Get search paths for custom themes in XDG directories.
    pub fn get_theme_search_paths(theme_name: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Search order:
        // 1. /usr/share/themes/<theme-name>/nptk-0/
        paths.push(PathBuf::from("/usr/share/themes").join(theme_name).join("nptk-0"));

        // 2. ~/.local/share/themes/<theme-name>/nptk-0/
        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                PathBuf::from(&home)
                    .join(".local")
                    .join("share")
                    .join("themes")
                    .join(theme_name)
                    .join("nptk-0"),
            );

            // 3. ~/themes/<theme-name>/nptk-0/
            paths.push(PathBuf::from(&home).join("themes").join(theme_name).join("nptk-0"));
        }

        paths
    }

    /// Find a theme directory for the given theme name.
    pub fn find_theme_directory(theme_name: &str) -> Option<PathBuf> {
        for path in Self::get_theme_search_paths(theme_name) {
            let theme_file = path.join("theme.toml");
            if theme_file.exists() {
                return Some(path);
            }
        }
        None
    }
}
