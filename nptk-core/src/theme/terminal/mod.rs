// SPDX-License-Identifier: LGPL-3.0-only

//! Terminal color scheme loading and management.
//!
//! This module handles terminal color schemes (ANSI colors) including:
//! - Terminal color structure and parsing from TOML
//! - Resolution logic for built-in vs custom schemes
//! - Built-in terminal color schemes

mod colors;
mod resolver;

pub use colors::TerminalColors;
pub use resolver::{resolve_terminal_colors, BuiltinTerminalSchemes};
