// SPDX-License-Identifier: LGPL-3.0-only

//! Theme role definitions.
//!
//! This module defines all the role enums used in the theme system.
//!
//! The module is split into separate files for each role type to improve
//! organization and maintainability.

mod color;
mod alignment;
mod flag;
mod metric;
mod path;
mod window;

pub use color::ColorRole;
pub use alignment::{AlignmentRole, TextAlignment};
pub use flag::FlagRole;
pub use metric::MetricRole;
pub use path::PathRole;
pub use window::WindowThemeProvider;

/// Macro to implement string conversion methods for role enums.
///
/// This macro generates `as_str()` and `from_str()` implementations
/// for role enums to reduce boilerplate.
#[macro_export]
macro_rules! impl_role_string_conversion {
    ($enum_name:ident, { $($variant:ident => $str:literal),* $(,)? }) => {
        impl $enum_name {
            /// Get the string representation of the role (for TOML keys).
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $str,)*
                }
            }
            
            /// Parse a role from a string (for TOML parsing).
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($str => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}
