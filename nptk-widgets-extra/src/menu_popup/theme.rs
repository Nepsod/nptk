// SPDX-License-Identifier: LGPL-3.0-only

//! Theme color extraction for menu popup widget
//!
//! Re-exports MenuThemeColors from nptk-core for consistency.
//! This alias is maintained for backward compatibility.

pub use nptk_core::menu::MenuThemeColors;

/// Alias for backward compatibility
/// 
/// @deprecated Use MenuThemeColors directly from nptk_core::menu instead
pub type ThemeColors = MenuThemeColors;
