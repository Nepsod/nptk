//! # Theme Rendering Bridge
//!
//! This module previously provided a bridge to the old centralized theme rendering system.
//! It has been removed in favor of the new role-based theming system using `Palette` from `nptk-core::theme`.
//!
//! Widgets should now access theme colors directly via `context.palette().color(ColorRole::...)` in their render methods.
