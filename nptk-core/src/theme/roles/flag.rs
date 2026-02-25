// SPDX-License-Identifier: LGPL-3.0-only

//! Flag roles for boolean theme properties.

/// Flag roles for boolean theme properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlagRole {
    /// Render bold text with a brighter color variant automatically.
    BoldTextAsBright,
    /// Only show icons on title buttons (hide text labels).
    TitleButtonsIconOnly,
}

crate::impl_role_string_conversion!(FlagRole, {
    BoldTextAsBright => "BoldTextAsBright",
    TitleButtonsIconOnly => "TitleButtonsIconOnly",
});
