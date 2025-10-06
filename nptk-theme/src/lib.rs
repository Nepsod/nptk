#![warn(missing_docs)]

//! # NPTK Theming System
//!
//! A comprehensive, type-safe theming system for the NPTK GUI toolkit.
//! This crate provides themes, styling structures, and utilities for creating
//! consistent, customizable user interfaces.
//!
//! ## Overview
//!
//! The theming system consists of several key components:
//!
//! - **[Theme](theme::Theme)**: The core trait that defines how themes work
//! - **[ThemeProperty](properties::ThemeProperty)**: Type-safe property keys
//! - **[ThemeManager](manager::ThemeManager)**: Runtime theme switching and caching
//! - **[ThemeHelper](helpers::ThemeHelper)**: Safe property access utilities
//! - **[ThemeConfig](config::ThemeConfig)**: Theme configuration from environment variables and files
//! - **Built-in Themes**: Light and dark theme implementations
//!
//! ## Quick Start
//!
//! ```rust
//! use nptk_theme::theme::dark::DarkTheme;
//! use nptk_theme::properties::ThemeProperty;
//! use nptk_theme::id::WidgetId;
//! use peniko::Color;
//!
//! // Create a theme
//! let theme = DarkTheme::new();
//!
//! // Get a theme property safely
//! let color = theme.get_property(
//!     WidgetId::new("nptk-widgets", "Button"),
//!     &ThemeProperty::ColorIdle
//! ).unwrap_or(Color::BLACK);
//! ```
//!
//! ## Type Safety
//!
//! The theming system uses enums instead of strings for property access,
//! providing compile-time safety and IDE support:
//!
//! ```rust
//! // ❌ Old way (unsafe)
//! let color = style.get_color("color_idle").unwrap(); // Could panic!
//!
//! // ✅ New way (safe)
//! let color = theme.get_property(widget_id, &ThemeProperty::ColorIdle)
//!     .unwrap_or(fallback_color);
//! ```
//!
//! ## Theme Configuration
//!
//! Configure themes through environment variables, configuration files, or programmatically:
//!
//! ```rust
//! use nptk_theme::config::ThemeConfig;
//!
//! // From environment variables
//! let config = ThemeConfig::from_env_or_default();
//! let theme = config.resolve_theme().unwrap();
//!
//! // From configuration file
//! let config = ThemeConfig::from_file("theme.toml").unwrap();
//! let theme = config.resolve_theme().unwrap();
//! ```
//!
//! ### Environment Variables
//!
//! Set themes via environment variables:
//!
//! ```bash
//! export NPTK_THEME=dark          # Use dark theme
//! export NPTK_THEME=light         # Use light theme
//! export NPTK_THEME=custom:my-theme  # Use custom theme
//! export NPTK_THEME_FALLBACK=light   # Set fallback theme
//! ```
//!
//! ## Theme Switching
//!
//! Switch themes at runtime without restarting the application:
//!
//! ```rust
//! use nptk_theme::manager::{ThemeManager, ThemeVariant};
//!
//! let mut manager = ThemeManager::new();
//! manager.switch_theme(&ThemeVariant::Dark);
//! ```
//!
//! ## CSS-Like Variables
//!
//! Use variables for consistent theming:
//!
//! ```rust
//! // Define variables
//! theme.variables_mut().set_color("primary", Color::from_rgb8(100, 150, 255));
//!
//! // Use variables
//! let primary_color = theme.variables().get_color("primary").unwrap();
//! ```
//!
//! ## Safe Property Access
//!
//! Use helpers for common patterns with automatic fallbacks:
//!
//! ```rust
//! use nptk_theme::helpers::{ThemeHelper, ButtonState};
//!
//! let color = ThemeHelper::get_button_color(
//!     theme,
//!     WidgetId::new("nptk-widgets", "Button"),
//!     ButtonState::Hovered,
//!     false // not focused
//! );
//! ```
//!
//! ## Architecture
//!
//! The theming system follows a layered architecture:
//!
//! 1. **Properties Layer**: Type-safe property definitions
//! 2. **Theme Layer**: Theme implementations and trait definitions
//! 3. **Manager Layer**: Runtime theme management and caching
//! 4. **Helper Layer**: Safe access utilities and common patterns
//!
//! ## Performance
//!
//! - **Caching**: Theme properties are cached for faster access
//! - **Type Safety**: Compile-time checks eliminate runtime errors
//! - **Memory Efficiency**: Enum-based properties reduce memory usage
//! - **Thread Safety**: Thread-safe theme management
//!
//! ## Migration Guide
//!
//! For detailed migration instructions from the old string-based system,
//! see the documentation for each module and the examples in the `examples/` directory.

/// Contains application integration traits and utilities.
pub mod app_integration;
/// Contains the [config::ThemeConfig] struct for theme configuration.
pub mod config;
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
/// Contains the [theme_resolver::SelfContainedThemeResolver] for self-contained theme resolution.
pub mod theme_resolver;
/// Contains centralized widget rendering functionality.
pub mod rendering;
