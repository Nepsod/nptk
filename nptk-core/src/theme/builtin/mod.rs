// SPDX-License-Identifier: LGPL-3.0-only

//! Built-in themes.
//!
//! Colors can use alpha transparency by using `Color::from_rgba8(r, g, b, a)`.
//! In TOML theme files, alpha colors are specified as 8-character hex strings:
//! - `#rrggbbaa` where `aa` is the alpha channel (0-255, 0x00-0xff)
//! - Example: `#ff000080` is red with 50% opacity

mod sweet;

pub use sweet::create_sweet_theme;

use vello::peniko::Color;
use super::ColorRole;
use super::Theme;
use super::util::rgba8;

/// Helper function to set a color with RGBA values.
pub(crate) fn set_color_rgba(theme: &mut Theme, role: ColorRole, r: u8, g: u8, b: u8, a: u8) {
    theme.set_color(role, rgba8(r, g, b, a));
}

/// Helper function to set a color with RGB values (alpha defaults to 255).
pub(crate) fn set_color_rgb(theme: &mut Theme, role: ColorRole, r: u8, g: u8, b: u8) {
    set_color_rgba(theme, role, r, g, b, 255);
}
