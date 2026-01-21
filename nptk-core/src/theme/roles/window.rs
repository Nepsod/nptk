// SPDX-License-Identifier: LGPL-3.0-only

//! Window theme provider types.

/// Window theme provider types (for future window manager integration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowThemeProvider {
    Classic,
    Redmond98,
    Silver,
    Sweet,
}

impl WindowThemeProvider {
    /// Parse window theme provider from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Classic" => Some(WindowThemeProvider::Classic),
            "Redmond98" => Some(WindowThemeProvider::Redmond98),
            "Silver" => Some(WindowThemeProvider::Silver),
            "Sweet" => Some(WindowThemeProvider::Sweet),
            _ => None,
        }
    }
}
