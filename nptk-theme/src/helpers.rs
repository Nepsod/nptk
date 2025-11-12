//! # Theme Helpers
//!
//! This module provides helper functions for safe theme property access with proper fallbacks.
//! These helpers simplify common theming patterns and ensure consistent behavior across widgets.
//!
//! ## Overview
//!
//! The helpers module provides:
//!
//! - **[ThemeHelper]**: Static helper functions for safe theme property access
//! - **State Enums**: Enums for common widget states (ButtonState, InputColorProperty, etc.)
//! - **Fallback Patterns**: Consistent fallback handling for missing theme properties
//!
//! ## Key Features
//!
//! - **Safe Property Access**: Automatic fallbacks for missing theme properties
//! - **State-Based Access**: Helper functions for common widget states
//! - **Type Safety**: Type-safe access to theme properties
//! - **Consistent Behavior**: Standardized fallback patterns across widgets
//! - **Performance**: Optimized for common use cases
//!
//! ## Usage Examples
//!
//! ### Basic Helper Usage
//!
//! ```rust
//! use nptk_theme::helpers::ThemeHelper;
//! use nptk_theme::properties::ThemeProperty;
//! use nptk_theme::id::WidgetId;
//! use peniko::Color;
//!
//! let theme = /* your theme */;
//! let widget_id = WidgetId::new("nptk-widgets", "Button");
//!
//! // Safe property access with fallback
//! let color = ThemeHelper::get_color_safe(
//!     &theme,
//!     widget_id,
//!     &ThemeProperty::ColorIdle,
//!     Color::from_rgb8(100, 150, 255) // fallback color
//! );
//! ```
//!
//! ### Button State Helpers
//!
//! ```rust
//! use nptk_theme::helpers::{ThemeHelper, ButtonState};
//! use nptk_theme::id::WidgetId;
//!
//! let theme = /* your theme */;
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//!
//! // Get button color based on state
//! let idle_color = ThemeHelper::get_button_color(
//!     &theme,
//!     button_id,
//!     ButtonState::Idle,
//!     false // not focused
//! );
//!
//! let hovered_color = ThemeHelper::get_button_color(
//!     &theme,
//!     button_id,
//!     ButtonState::Hovered,
//!     false // not focused
//! );
//!
//! let focused_color = ThemeHelper::get_button_color(
//!     &theme,
//!     button_id,
//!     ButtonState::Idle,
//!     true // focused
//! );
//! ```
//!
//! ### Input Field Helpers
//!
//! ```rust
//! use nptk_theme::helpers::{ThemeHelper, InputColorProperty};
//! use nptk_theme::id::WidgetId;
//!
//! let theme = /* your theme */;
//! let input_id = WidgetId::new("nptk-widgets", "TextInput");
//!
//! // Get input colors based on state
//! let background_color = ThemeHelper::get_input_color(
//!     &theme,
//!     input_id,
//!     InputColorProperty::Background,
//!     true,  // is focused
//!     true   // is valid
//! );
//!
//! let border_color = ThemeHelper::get_input_color(
//!     &theme,
//!     input_id,
//!     InputColorProperty::Border,
//!     true,  // is focused
//!     false  // is valid (shows error state)
//! );
//! ```
//!
//! ### Checkbox Helpers
//!
//! ```rust
//! use nptk_theme::helpers::ThemeHelper;
//! use nptk_theme::id::WidgetId;
//!
//! let theme = /* your theme */;
//! let checkbox_id = WidgetId::new("nptk-widgets", "Checkbox");
//!
//! // Get checkbox colors based on state
//! let checked_color = ThemeHelper::get_checkbox_color(
//!     &theme,
//!     checkbox_id,
//!     true // is checked
//! );
//!
//! let unchecked_color = ThemeHelper::get_checkbox_color(
//!     &theme,
//!     checkbox_id,
//!     false // is not checked
//! );
//! ```
//!
//! ### Progress Bar Helpers
//!
//! ```rust
//! use nptk_theme::helpers::{ThemeHelper, ProgressColorProperty};
//! use nptk_theme::id::WidgetId;
//!
//! let theme = /* your theme */;
//! let progress_id = WidgetId::new("nptk-widgets", "Progress");
//!
//! // Get progress bar colors
//! let background_color = ThemeHelper::get_progress_color(
//!     &theme,
//!     progress_id,
//!     ProgressColorProperty::Background
//! );
//!
//! let progress_color = ThemeHelper::get_progress_color(
//!     &theme,
//!     progress_id,
//!     ProgressColorProperty::Progress
//! );
//!
//! let border_color = ThemeHelper::get_progress_color(
//!     &theme,
//!     progress_id,
//!     ProgressColorProperty::Border
//! );
//! ```
//!
//! ### Multiple Fallbacks
//!
//! ```rust
//! use nptk_theme::helpers::ThemeHelper;
//! use nptk_theme::properties::ThemeProperty;
//! use nptk_theme::id::WidgetId;
//! use peniko::Color;
//!
//! let theme = /* your theme */;
//! let widget_id = WidgetId::new("nptk-widgets", "Button");
//!
//! // Multiple fallback options
//! let fallbacks = [
//!     Color::from_rgb8(100, 150, 255), // primary fallback
//!     Color::from_rgb8(200, 200, 200), // secondary fallback
//!     Color::BLACK,                    // final fallback
//! ];
//!
//! let color = ThemeHelper::get_color_with_fallbacks(
//!     &theme,
//!     widget_id,
//!     &ThemeProperty::ColorIdle,
//!     &fallbacks
//! );
//! ```
//!
//! ## State Enums
//!
//! ### ButtonState
//!
//! Represents the different states a button can be in:
//!
//! ```rust
//! use nptk_theme::helpers::ButtonState;
//!
//! let states = [
//!     ButtonState::Idle,      // Normal state
//!     ButtonState::Hovered,   // Mouse over
//!     ButtonState::Pressed,   // Being pressed
//!     ButtonState::Released,  // Just released
//! ];
//! ```
//!
//! ### InputColorProperty
//!
//! Represents different color properties for input fields:
//!
//! ```rust
//! use nptk_theme::helpers::InputColorProperty;
//!
//! let properties = [
//!     InputColorProperty::Background,  // Background color
//!     InputColorProperty::Border,      // Border color
//!     InputColorProperty::Text,        // Text color
//!     InputColorProperty::Cursor,      // Cursor color
//!     InputColorProperty::Selection,   // Selection color
//!     InputColorProperty::Placeholder, // Placeholder text color
//! ];
//! ```
//!
//! ### ProgressColorProperty
//!
//! Represents different color properties for progress bars:
//!
//! ```rust
//! use nptk_theme::helpers::ProgressColorProperty;
//!
//! let properties = [
//!     ProgressColorProperty::Background, // Background color
//!     ProgressColorProperty::Progress,   // Progress fill color
//!     ProgressColorProperty::Border,     // Border color
//! ];
//! ```
//!
//! ## Best Practices
//!
//! 1. **Use Helpers**: Always use helper functions for theme property access
//! 2. **Provide Fallbacks**: Always provide sensible fallback values
//! 3. **Use State Enums**: Use the provided state enums for consistency
//! 4. **Handle Errors**: Always handle potential errors gracefully
//! 5. **Document Usage**: Document any custom helper usage patterns
//!
//! ## Performance Considerations
//!
//! - **Caching**: Helpers work with theme caching for optimal performance
//! - **Type Safety**: Compile-time checks eliminate runtime errors
//! - **Efficient Fallbacks**: Fallback logic is optimized for common cases
//! - **Memory Usage**: Minimal memory overhead for helper functions

use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::Theme;
use peniko::Color;

/// The state of a checkbox widget (duplicated from nptk-widgets to avoid circular dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxState {
    /// Unchecked state
    Unchecked,
    /// Checked state  
    Checked,
    /// Indeterminate state (partially selected, like in Windows file trees)
    Indeterminate,
}

/// Helper functions for safe theme property access with proper fallbacks.
///
/// This struct provides static helper functions that simplify common theming patterns
/// and ensure consistent behavior across widgets. All helper functions provide
/// automatic fallbacks for missing theme properties.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::helpers::ThemeHelper;
/// use nptk_theme::properties::ThemeProperty;
/// use nptk_theme::id::WidgetId;
/// use peniko::Color;
///
/// let theme = /* your theme */;
/// let widget_id = WidgetId::new("nptk-widgets", "Button");
///
/// // Safe property access with fallback
/// let color = ThemeHelper::get_color_safe(
///     &theme,
///     widget_id,
///     &ThemeProperty::ColorIdle,
///     Color::from_rgb8(100, 150, 255) // fallback color
/// );
/// ```
///
/// # Key Features
///
/// - **Safe Access**: Automatic fallbacks for missing properties
/// - **Type Safety**: Type-safe access to theme properties
/// - **Consistent Behavior**: Standardized fallback patterns
/// - **Performance**: Optimized for common use cases
/// - **State Support**: Helper functions for common widget states
pub struct ThemeHelper;

impl ThemeHelper {
    /// Get a color property with safe fallbacks.
    /// This is the recommended way to access theme colors in widgets.
    pub fn get_color_safe<T: Theme + ?Sized>(
        theme: &T,
        widget_id: WidgetId,
        property: &ThemeProperty,
        fallback: Color,
    ) -> Color {
        theme.get_property(widget_id, property).unwrap_or(fallback)
    }

    /// Get a color property with multiple fallback options.
    pub fn get_color_with_fallbacks<T: Theme>(
        theme: &T,
        widget_id: WidgetId,
        property: &ThemeProperty,
        fallbacks: &[Color],
    ) -> Color {
        if let Some(color) = theme.get_property(widget_id, property) {
            return color;
        }

        // Try default property
        if let Some(color) = theme.get_default_property(property) {
            return color;
        }

        // Use first fallback or black as last resort
        fallbacks.first().copied().unwrap_or(Color::BLACK)
    }

    /// Get a button color based on state with safe fallbacks.
    pub fn get_button_color<T: Theme>(
        theme: &T,
        widget_id: WidgetId,
        state: ButtonState,
        is_focused: bool,
    ) -> Color {
        let property = if is_focused {
            ThemeProperty::ColorFocused
        } else {
            match state {
                ButtonState::Idle => ThemeProperty::ColorIdle,
                ButtonState::Hovered => ThemeProperty::ColorHovered,
                ButtonState::Pressed => ThemeProperty::ColorPressed,
                ButtonState::Released => ThemeProperty::ColorHovered,
            }
        };

        Self::get_color_safe(theme, widget_id, &property, Color::from_rgb8(150, 170, 250))
    }

    /// Get an input field color based on state with safe fallbacks.
    pub fn get_input_color<T: Theme>(
        theme: &T,
        widget_id: WidgetId,
        property: InputColorProperty,
        is_focused: bool,
        is_valid: bool,
    ) -> Color {
        let theme_property = match property {
            InputColorProperty::Background => {
                if is_focused {
                    ThemeProperty::ColorBackgroundFocused
                } else {
                    ThemeProperty::ColorBackground
                }
            },
            InputColorProperty::Border => {
                if !is_valid {
                    ThemeProperty::ColorBorderError
                } else if is_focused {
                    ThemeProperty::ColorBorderFocused
                } else {
                    ThemeProperty::ColorBorder
                }
            },
            InputColorProperty::Text => ThemeProperty::ColorText,
            InputColorProperty::Cursor => ThemeProperty::ColorCursor,
            InputColorProperty::Selection => ThemeProperty::ColorSelection,
            InputColorProperty::Placeholder => ThemeProperty::ColorPlaceholder,
        };

        let fallback = match property {
            InputColorProperty::Background => {
                if is_focused {
                    Color::WHITE
                } else {
                    Color::from_rgb8(240, 240, 240)
                }
            },
            InputColorProperty::Border => {
                if !is_valid {
                    Color::from_rgb8(255, 0, 0)
                } else if is_focused {
                    Color::from_rgb8(0, 120, 255)
                } else {
                    Color::from_rgb8(200, 200, 200)
                }
            },
            InputColorProperty::Text => Color::BLACK,
            InputColorProperty::Cursor => Color::BLACK,
            InputColorProperty::Selection => Color::from_rgb8(180, 200, 255),
            InputColorProperty::Placeholder => Color::from_rgb8(150, 150, 150),
        };

        Self::get_color_safe(theme, widget_id, &theme_property, fallback)
    }

    /// Get a checkbox color based on state with safe fallbacks.
    pub fn get_checkbox_color<T: Theme>(theme: &T, widget_id: WidgetId, is_checked: bool) -> Color {
        let property = if is_checked {
            ThemeProperty::ColorChecked
        } else {
            ThemeProperty::ColorUnchecked
        };

        let fallback = if is_checked {
            Color::from_rgb8(130, 130, 230)
        } else {
            Color::from_rgb8(170, 170, 250)
        };

        Self::get_color_safe(theme, widget_id, &property, fallback)
    }

    /// Get a checkbox color based on three-state checkbox state with safe fallbacks.
    pub fn get_checkbox_color_three_state<T: Theme + ?Sized>(
        theme: &T,
        widget_id: WidgetId,
        state: CheckboxState,
    ) -> Color {
        let property = match state {
            CheckboxState::Unchecked => ThemeProperty::ColorUnchecked,
            CheckboxState::Checked => ThemeProperty::ColorChecked,
            CheckboxState::Indeterminate => ThemeProperty::ColorIndeterminate,
        };

        let fallback = match state {
            CheckboxState::Unchecked => Color::from_rgb8(170, 170, 250),
            CheckboxState::Checked => Color::from_rgb8(130, 130, 230),
            CheckboxState::Indeterminate => Color::from_rgb8(150, 150, 240),
        };

        Self::get_color_safe(theme, widget_id, &property, fallback)
    }

    /// Get a progress bar color with safe fallbacks.
    pub fn get_progress_color<T: Theme>(
        theme: &T,
        widget_id: WidgetId,
        property: ProgressColorProperty,
    ) -> Color {
        let theme_property = match property {
            ProgressColorProperty::Background => ThemeProperty::Background,
            ProgressColorProperty::Progress => ThemeProperty::ColorProgress,
            ProgressColorProperty::Border => ThemeProperty::Border,
        };

        let fallback = match property {
            ProgressColorProperty::Background => Color::from_rgb8(220, 220, 220),
            ProgressColorProperty::Progress => Color::from_rgb8(100, 150, 255),
            ProgressColorProperty::Border => Color::from_rgb8(180, 180, 180),
        };

        Self::get_color_safe(theme, widget_id, &theme_property, fallback)
    }
}

/// Button state for theme color selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Button in normal, idle state.
    Idle,
    /// Button when mouse is hovering over it.
    Hovered,
    /// Button when being pressed down.
    Pressed,
    /// Button when just released after being pressed.
    Released,
}

/// Input field color properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputColorProperty {
    /// Input field background color.
    Background,
    /// Input field border color.
    Border,
    /// Input field text color.
    Text,
    /// Input field cursor color.
    Cursor,
    /// Input field text selection color.
    Selection,
    /// Input field placeholder text color.
    Placeholder,
}

/// Progress bar color properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressColorProperty {
    /// Progress bar background color.
    Background,
    /// Progress bar fill color.
    Progress,
    /// Progress bar border color.
    Border,
}
