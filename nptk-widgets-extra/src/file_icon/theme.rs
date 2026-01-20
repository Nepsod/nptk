// SPDX-License-Identifier: LGPL-3.0-only
//! Theme-related helpers for file icon widget.

use nptk_core::theme::{ColorRole, Palette};
use nptk_core::vg::peniko::Color;

/// Extract icon color from palette.
pub fn get_icon_color(palette: &Palette) -> Color {
    palette.color(ColorRole::BaseText)
}
