// SPDX-License-Identifier: LGPL-3.0-only

//! Alignment roles and text alignment values.

/// Alignment roles for text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignmentRole {
    /// Alignment of text inside the window title bar.
    TitleAlignment,
}

crate::impl_role_string_conversion!(AlignmentRole, {
    TitleAlignment => "TitleAlignment",
});

/// Text alignment values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    /// Align left.
    Left,
    /// Align center.
    Center,
    /// Align right.
    Right,
}

impl TextAlignment {
    /// Parse text alignment from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "left" | "centerleft" => Some(TextAlignment::Left),
            "center" => Some(TextAlignment::Center),
            "right" | "centerright" => Some(TextAlignment::Right),
            _ => None,
        }
    }
}
