#![warn(missing_docs)]

//! Theme/Styling library for nptk => See `nptk` crate.
//!
//! Contains themes and widget styles.

/// Contains the [globals::Globals] struct.
pub mod globals;
/// Contains helper functions for safe theme property access.
pub mod helpers;
/// Contains the [id::WidgetId] struct.
pub mod id;
/// Contains theme management and runtime switching.
pub mod manager;
/// Contains type-safe theme properties and values.
pub mod properties;
/// Contains styling structures.
pub mod style;
/// Contains the [theme::Theme] trait and built-in themes.
pub mod theme;
