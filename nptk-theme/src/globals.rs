//! # Global Theme Values
//!
//! This module provides global theme values that can be used across all widgets
//! in a theme. These values represent theme-wide settings that affect the overall
//! appearance and behavior of the application.
//!
//! ## Overview
//!
//! The globals module provides:
//!
//! - **[Globals]**: Global theme settings and values
//! - **Theme-wide Configuration**: Settings that affect all widgets
//! - **Consistent Behavior**: Standardized global theme behavior
//!
//! ## Usage Examples
//!
//! ### Basic Global Usage
//!
//! ```rust
//! use nptk_theme::globals::Globals;
//!
//! // Create global settings
//! let mut globals = Globals::default();
//! globals.invert_text_color = true;
//!
//! // Use in theme
//! let theme = MyTheme::new(globals);
//! ```
//!
//! ### Theme Integration
//!
//! ```rust
//! use nptk_theme::globals::Globals;
//! use nptk_theme::theme::Theme;
//!
//! impl Theme for MyTheme {
//!     fn globals(&self) -> &Globals {
//!         &self.globals
//!     }
//!
//!     fn globals_mut(&mut self) -> &mut Globals {
//!         &mut self.globals
//!     }
//! }
//! ```
//!
//! ### Widget Usage
//!
//! ```rust
//! use nptk_theme::globals::Globals;
//!
//! // In widget rendering
//! let text_color = if theme.globals().invert_text_color {
//!     // Use inverted color
//!     Color::WHITE
//! } else {
//!     // Use normal color
//!     Color::BLACK
//! };
//! ```
//!
//! ## Global Properties
//!
//! ### Text Color Inversion
//!
//! The `invert_text_color` property controls whether text colors should be inverted
//! for better contrast in certain contexts (e.g., when widgets are inside dark containers):
//!
//! ```rust
//! use nptk_theme::globals::Globals;
//!
//! let mut globals = Globals::default();
//!
//! // Enable text color inversion
//! globals.invert_text_color = true;
//!
//! // This will cause widgets to use inverted text colors
//! // for better contrast in dark contexts
//! ```
//!
//! ## Best Practices
//!
//! 1. **Use Sparingly**: Only use globals for truly global settings
//! 2. **Document Usage**: Document what each global property does
//! 3. **Consistent Behavior**: Ensure globals behave consistently across widgets
//! 4. **Performance**: Globals are copied, so keep them lightweight
//! 5. **Thread Safety**: Globals implement Copy, so they're thread-safe
//!
//! ## Performance Considerations
//!
//! - **Copy Semantics**: Globals implement Copy for efficient storage
//! - **Lightweight**: Minimal memory footprint
//! - **Fast Access**: Direct field access with no indirection
//! - **Thread Safe**: Copy semantics make globals thread-safe

/// Global theme values for all widgets to use.
///
/// This struct contains theme-wide settings that affect the overall appearance
/// and behavior of the application. These values are accessible to all widgets
/// and provide a way to implement global theme features.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::globals::Globals;
///
/// // Create global settings
/// let mut globals = Globals::default();
/// globals.invert_text_color = true;
///
/// // Use in theme
/// let theme = MyTheme::new(globals);
/// ```
///
/// # Properties
///
/// ## Text Color Inversion
///
/// The `invert_text_color` property controls whether text colors should be inverted
/// for better contrast in certain contexts. This is useful when widgets are placed
/// inside dark containers or when the overall theme context requires inverted text.
///
/// ```rust
/// use nptk_theme::globals::Globals;
///
/// let mut globals = Globals::default();
///
/// // Enable text color inversion
/// globals.invert_text_color = true;
///
/// // This will cause widgets to use inverted text colors
/// // for better contrast in dark contexts
/// ```
///
/// # Performance
///
/// - **Copy Semantics**: Implements Copy for efficient storage and passing
/// - **Lightweight**: Minimal memory footprint
/// - **Fast Access**: Direct field access with no indirection
/// - **Thread Safe**: Copy semantics make globals thread-safe
///
/// # Best Practices
///
/// 1. **Use Sparingly**: Only use globals for truly global settings
/// 2. **Document Usage**: Document what each global property does
/// 3. **Consistent Behavior**: Ensure globals behave consistently across widgets
/// 4. **Performance**: Keep globals lightweight since they're copied
/// 5. **Thread Safety**: Globals are thread-safe due to Copy semantics
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Globals {
    /// Invert text color for better contrast in dark contexts.
    ///
    /// When `true`, widgets will use inverted text colors for better contrast
    /// in dark containers or when the overall theme context requires it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nptk_theme::globals::Globals;
    ///
    /// let mut globals = Globals::default();
    /// globals.invert_text_color = true;
    ///
    /// // Widgets will now use inverted text colors
    /// ```
    pub invert_text_color: bool,
}
