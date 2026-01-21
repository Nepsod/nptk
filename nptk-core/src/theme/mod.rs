// SPDX-License-Identifier: LGPL-3.0-only

//! Theme system for NPTK.
//!
//! This module provides a role-based theming system,
//! where widgets use semantic color roles instead of per-widget properties.
//!
//! ## Overview
//!
//! The theme system consists of:
//!
//! - **[Theme]**: Main theme structure containing role-based data
//! - **[Palette]**: Widget-facing API for accessing theme data
//! - **[ColorRole]**, **[AlignmentRole]**, **[FlagRole]**, etc.: Role enums for theme properties
//! - **[ThemeLoader]**: Async loader for TOML theme files
//! - **[ThemeResolver]**: Resolver for built-in vs custom themes
//! - **[TerminalColors]**: Terminal color scheme management
//!
//! ## Usage
//!
//! ```rust
//! use nptk_core::theme::{ThemeResolver, Palette};
//!
//! // Load a theme
//! let theme = ThemeResolver::resolve("Sweet").await?;
//! let palette = Palette::new(theme);
//!
//! // Use the palette in widgets
//! let color = palette.color(ColorRole::Button);
//! ```

mod error;
mod roles;
mod palette;
mod loader;
mod resolver;
mod terminal;
mod builtin;
mod util;

pub use error::ThemeError;
pub use roles::{
    AlignmentRole, ColorRole, FlagRole, MetricRole, PathRole, TextAlignment, WindowThemeProvider,
};
pub use palette::Palette;
pub use loader::ThemeLoader;
pub use resolver::ThemeResolver;
pub use terminal::{TerminalColors, resolve_terminal_colors};
pub use builtin::create_sweet_theme;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use vello::peniko::Color;
use crate::theme::util::rgba8;

/// Main theme structure containing all role-based theme data.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Color roles mapped to peniko::Color values.
    colors: HashMap<ColorRole, Color>,
    /// Alignment roles mapped to TextAlignment values.
    alignments: HashMap<AlignmentRole, TextAlignment>,
    /// Flag roles mapped to boolean values.
    flags: HashMap<FlagRole, bool>,
    /// Metric roles mapped to integer values.
    metrics: HashMap<MetricRole, i32>,
    /// Path roles mapped to PathBuf values.
    paths: HashMap<PathRole, PathBuf>,
    /// Optional terminal colors.
    terminal_colors: Option<TerminalColors>,
    /// Window theme provider (for future window manager integration).
    window_theme: Option<WindowThemeProvider>,
}

impl Theme {
    /// Create a new empty theme.
    pub fn new() -> Self {
        Self {
            colors: HashMap::new(),
            alignments: HashMap::new(),
            flags: HashMap::new(),
            metrics: HashMap::new(),
            paths: HashMap::new(),
            terminal_colors: None,
            window_theme: None,
        }
    }

    /// Set a color role.
    pub fn set_color(&mut self, role: ColorRole, color: Color) {
        self.colors.insert(role, color);
    }

    /// Get a color role, with fallback to default.
    pub fn color(&self, role: ColorRole) -> Color {
        self.colors
            .get(&role)
            .copied()
            .unwrap_or_else(|| Self::default_color(role))
    }

    /// Set an alignment role.
    pub fn set_alignment(&mut self, role: AlignmentRole, alignment: TextAlignment) {
        self.alignments.insert(role, alignment);
    }

    /// Get an alignment role, with fallback to default.
    pub fn alignment(&self, role: AlignmentRole) -> TextAlignment {
        self.alignments
            .get(&role)
            .copied()
            .unwrap_or(TextAlignment::Left)
    }

    /// Set a flag role.
    pub fn set_flag(&mut self, role: FlagRole, value: bool) {
        self.flags.insert(role, value);
    }

    /// Get a flag role, with fallback to default.
    pub fn flag(&self, role: FlagRole) -> bool {
        self.flags.get(&role).copied().unwrap_or(false)
    }

    /// Set a metric role.
    pub fn set_metric(&mut self, role: MetricRole, value: i32) {
        self.metrics.insert(role, value);
    }

    /// Get a metric role, with fallback to default.
    pub fn metric(&self, role: MetricRole) -> i32 {
        self.metrics
            .get(&role)
            .copied()
            .unwrap_or_else(|| role.default_value())
    }

    /// Set a path role.
    pub fn set_path(&mut self, role: PathRole, path: PathBuf) {
        self.paths.insert(role, path);
    }

    /// Get a path role, with fallback to default.
    pub fn path(&self, role: PathRole) -> Option<&PathBuf> {
        self.paths.get(&role)
    }

    /// Set terminal colors.
    pub fn set_terminal_colors(&mut self, colors: TerminalColors) {
        self.terminal_colors = Some(colors);
    }

    /// Get terminal colors.
    pub fn terminal_colors(&self) -> Option<&TerminalColors> {
        self.terminal_colors.as_ref()
    }

    /// Set window theme provider.
    pub fn set_window_theme(&mut self, provider: WindowThemeProvider) {
        self.window_theme = Some(provider);
    }

    /// Get window theme provider.
    pub fn window_theme(&self) -> Option<WindowThemeProvider> {
        self.window_theme
    }

    /// Get default color for a role (used as fallback).
    fn default_color(role: ColorRole) -> Color {
        match role {
            ColorRole::Window => rgba8(50, 50, 50, 255),
            ColorRole::WindowText => rgba8(255, 255, 255, 255),
            ColorRole::Button => rgba8(85, 85, 85, 255),
            ColorRole::ButtonText => rgba8(255, 255, 255, 255),
            ColorRole::Base => rgba8(66, 66, 66, 255),
            ColorRole::BaseText => rgba8(255, 255, 255, 255),
            ColorRole::Selection => rgba8(100, 150, 255, 255),
            ColorRole::SelectionText => rgba8(255, 255, 255, 255),
            ColorRole::Black => rgba8(0, 0, 0, 255),
            ColorRole::White => rgba8(255, 255, 255, 255),
            ColorRole::Red => rgba8(204, 0, 0, 255),
            ColorRole::Green => rgba8(62, 154, 6, 255),
            ColorRole::Blue => rgba8(52, 101, 164, 255),
            ColorRole::Yellow => rgba8(196, 160, 0, 255),
            ColorRole::Magenta => rgba8(117, 80, 123, 255),
            ColorRole::Cyan => rgba8(6, 152, 154, 255),
            _ => rgba8(0, 0, 0, 255),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
