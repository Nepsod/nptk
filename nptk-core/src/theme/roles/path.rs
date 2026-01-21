// SPDX-License-Identifier: LGPL-3.0-only

//! Path roles for file path theme properties.

/// Path roles for file path theme properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathRole {
    TitleButtonIcons,
    // Shadow paths are TODO for future kurbo-based implementation
    // ActiveWindowShadow,
    // InactiveWindowShadow,
    // TaskbarShadow,
    // MenuShadow,
    // TooltipShadow,
    // OverlayRectShadow,
}

crate::impl_role_string_conversion!(PathRole, {
    TitleButtonIcons => "TitleButtonIcons",
});

impl PathRole {
    /// Get the default value for a path role.
    pub fn default_value(&self) -> &'static str {
        match self {
            PathRole::TitleButtonIcons => "/res/icons/16x16/",
        }
    }
}
