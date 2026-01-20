// SPDX-License-Identifier: LGPL-3.0-only

//! Theme system for NPTK.
//!
//! This module provides a role-based theming system,
//! where widgets use semantic color roles instead of per-widget properties.

mod error;
mod roles;
mod palette;
mod loader;
mod resolver;
mod terminal;
mod builtin;

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
            ColorRole::Window => Color::from_rgb8(50, 50, 50),
            ColorRole::WindowText => Color::WHITE,
            ColorRole::Button => Color::from_rgb8(85, 85, 85),
            ColorRole::ButtonText => Color::WHITE,
            ColorRole::Base => Color::from_rgb8(66, 66, 66),
            ColorRole::BaseText => Color::WHITE,
            ColorRole::Selection => Color::from_rgb8(100, 150, 255),
            ColorRole::SelectionText => Color::WHITE,
            ColorRole::Black => Color::from_rgb8(0, 0, 0),
            ColorRole::White => Color::from_rgb8(255, 255, 255),
            ColorRole::Red => Color::from_rgb8(204, 0, 0),
            ColorRole::Green => Color::from_rgb8(62, 154, 6),
            ColorRole::Blue => Color::from_rgb8(52, 101, 164),
            ColorRole::Yellow => Color::from_rgb8(196, 160, 0),
            ColorRole::Magenta => Color::from_rgb8(117, 80, 123),
            ColorRole::Cyan => Color::from_rgb8(6, 152, 154),
            _ => Color::BLACK,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
