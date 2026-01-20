// SPDX-License-Identifier: LGPL-3.0-only

//! Palette API for widgets.
//!
//! The Palette provides a widget-facing API to access theme colors, metrics,
//! flags, and paths. It wraps a shared Theme and provides convenient methods.

use std::path::Path;
use std::sync::Arc;
use vello::peniko::Color;
use super::{AlignmentRole, ColorRole, FlagRole, MetricRole, PathRole, TextAlignment, Theme};

/// Palette provides widget-facing API for accessing theme data.
///
/// Widgets use the Palette to get colors, alignments, flags, metrics, and paths
/// from the current theme. The Palette wraps a shared Theme (via Arc) to avoid
/// cloning the entire theme data.
#[derive(Debug, Clone)]
pub struct Palette {
    theme: Arc<Theme>,
}

impl Palette {
    /// Create a new Palette from a Theme.
    pub fn new(theme: Theme) -> Self {
        Self {
            theme: Arc::new(theme),
        }
    }

    /// Create a Palette from an Arc<Theme>.
    pub fn from_arc(theme: Arc<Theme>) -> Self {
        Self { theme }
    }

    /// Get a color for the given role.
    ///
    /// Returns the color for the role, or a default color if not set.
    pub fn color(&self, role: ColorRole) -> Color {
        self.theme.color(role)
    }

    /// Get an alignment for the given role.
    ///
    /// Returns the alignment for the role, or Left as default.
    pub fn alignment(&self, role: AlignmentRole) -> TextAlignment {
        self.theme.alignment(role)
    }

    /// Get a flag value for the given role.
    ///
    /// Returns the flag value, or false as default.
    pub fn flag(&self, role: FlagRole) -> bool {
        self.theme.flag(role)
    }

    /// Get a metric value for the given role.
    ///
    /// Returns the metric value, or the role's default value.
    pub fn metric(&self, role: MetricRole) -> i32 {
        self.theme.metric(role)
    }

    /// Get a path for the given role.
    ///
    /// Returns Some(path) if the role has a path set, None otherwise.
    pub fn path(&self, role: PathRole) -> Option<&Path> {
        self.theme.path(role).map(|p| p.as_path())
    }

    /// Get the underlying theme (for advanced use cases).
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Get terminal colors if available.
    pub fn terminal_colors(&self) -> Option<&super::TerminalColors> {
        self.theme.terminal_colors()
    }
}
