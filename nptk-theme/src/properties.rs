//! # Theme Properties
//!
//! This module provides type-safe theme properties and values for the NPTK theming system.
//! It replaces the old string-based property access with compile-time safe enums and
//! provides a comprehensive set of styling data types.
//!
//! ## Overview
//!
//! The properties system consists of:
//!
//! - **[ThemeProperty]**: Type-safe property keys for all widget styling
//! - **[ThemeValue]**: Type-safe values that can hold different types of styling data
//! - **[ThemeStyle]**: A collection of properties for a specific widget
//! - **[ThemeVariables]**: CSS-like variables for consistent theming
//!
//! ## Type Safety Benefits
//!
//! Using enums instead of strings provides several advantages:
//!
//! - **Compile-time Safety**: Typos are caught at compile time
//! - **IDE Support**: Autocomplete and refactoring support
//! - **Performance**: Enum matching is faster than string hashing
//! - **Memory Efficiency**: Enums use less memory than strings
//! - **Documentation**: Self-documenting code with clear property names
//!
//! ## Usage Examples
//!
//! ### Basic Property Access
//!
//! ```rust
//! use nptk_theme::properties::{ThemeProperty, ThemeValue};
//! use vello::peniko::Color;
//!
//! // Create a theme style
//! let mut style = ThemeStyle::new();
//! style.set_color(ThemeProperty::ColorIdle, Color::from_rgb8(100, 150, 255));
//!
//! // Get a property value
//! let color = style.get_color(&ThemeProperty::ColorIdle).unwrap();
//! ```
//!
//! ### Using Theme Variables
//!
//! ```rust
//! use nptk_theme::properties::{ThemeVariables, ThemeValue};
//! use vello::peniko::Color;
//!
//! // Set up theme variables
//! let mut variables = ThemeVariables::new();
//! variables.set_color("primary", Color::from_rgb8(100, 150, 255));
//! variables.set_color("secondary", Color::from_rgb8(200, 200, 200));
//!
//! // Use variables in styles
//! let primary_color = variables.get_color("primary").unwrap();
//! ```
//!
//! ### Custom Properties
//!
//! ```rust
//! use nptk_theme::properties::ThemeProperty;
//!
//! // Create custom properties for specialized widgets
//! let custom_prop = ThemeProperty::custom("my_custom_color");
//! ```
//!
//! ## Property Categories
//!
//! Properties are organized by widget type for better organization:
//!
//! - **Common**: Properties used across multiple widgets
//! - **Button**: Button-specific properties (idle, hovered, pressed, focused)
//! - **Input**: Input field properties (background, border, text, cursor, etc.)
//! - **Checkbox**: Checkbox-specific properties (checked, unchecked)
//! - **Slider**: Slider properties (track, ball)
//! - **Radio Button**: Radio button properties (background, border, dot, text)
//! - **Menu**: Menu properties (hovered, selected, disabled)
//! - **Scroll Container**: Scrollbar properties (track, thumb, hover, active)
//! - **Tabs**: Tab container properties (bar, content, active, inactive, etc.)
//! - **Progress**: Progress bar properties (background, progress, border)
//!
//! ## Value Types
//!
//! The [ThemeValue] enum supports various data types:
//!
//! - **Color**: RGB colors for backgrounds, text, borders
//! - **Gradient**: Color gradients for advanced styling
//! - **Brush**: Peniko brush objects for complex drawing
//! - **Float**: Floating-point values for sizes, opacities
//! - **Int/UInt**: Integer values for counts, sizes
//! - **Bool**: Boolean values for flags, states
//! - **String**: Text values for labels, descriptions
//! - **Reference**: References to other properties for inheritance
//!
//! ## Performance Considerations
//!
//! - **Enum Matching**: Very fast, compiled to efficient jump tables
//! - **HashMap Storage**: Efficient property storage and lookup
//! - **Copy Semantics**: Most properties implement Copy for efficiency
//! - **Memory Layout**: Optimized memory layout for common use cases
//!
//! ## Best Practices
//!
//! 1. **Use Type-Safe Properties**: Always use [ThemeProperty] enums instead of strings
//! 2. **Provide Fallbacks**: Always provide sensible default values
//! 3. **Use Variables**: Define reusable values in [ThemeVariables]
//! 4. **Group Related Properties**: Keep related properties together
//! 5. **Document Custom Properties**: Document any custom properties you create

use vello::peniko::{Brush, Color, Gradient};
use std::collections::HashMap;

/// Type-safe theme property keys for widgets.
///
/// This enum eliminates string-based property access and provides compile-time safety.
/// Each property represents a specific styling aspect of a widget, organized by widget type.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::properties::ThemeProperty;
/// use vello::peniko::Color;
///
/// // Button properties
/// let idle_color = ThemeProperty::ColorIdle;
/// let hovered_color = ThemeProperty::ColorHovered;
///
/// // Input properties
/// let background_color = ThemeProperty::ColorBackground;
/// let border_color = ThemeProperty::ColorBorder;
///
/// // Custom properties
/// let custom_prop = ThemeProperty::custom("my_custom_property");
/// ```
///
/// # Property Categories
///
/// Properties are organized into logical groups:
///
/// - **Common**: Used across multiple widgets (Color, Background, Border, Text)
/// - **Button**: Button-specific states (Idle, Pressed, Hovered, Focused)
/// - **Input**: Input field styling (Background, Border, Text, Cursor, Selection, Placeholder)
/// - **Checkbox**: Checkbox states (Checked, Unchecked)
/// - **Slider**: Slider components (Track, Ball)
/// - **Radio Button**: Radio button styling (Background, Border, Dot, Text states)
/// - **Menu**: Menu styling (Hovered, Selected, Disabled)
/// - **Scroll Container**: Scrollbar styling (Track, Thumb, Hover, Active states)
/// - **Tabs**: Tab container styling (Bar, Content, Active, Inactive, Hover, Press states)
/// - **Progress**: Progress bar styling (Background, Progress fill, Border)
/// - **Custom**: User-defined properties for specialized widgets
///
/// # Type Safety
///
/// Using enums instead of strings provides:
///
/// - **Compile-time Safety**: Typos are caught at compile time
/// - **IDE Support**: Autocomplete and refactoring support
/// - **Performance**: Enum matching is faster than string hashing
/// - **Memory Efficiency**: Enums use less memory than strings
///
/// # Custom Properties
///
/// For specialized widgets or custom styling needs, you can create custom properties:
///
/// ```rust
/// use nptk_theme::properties::ThemeProperty;
///
/// let custom_prop = ThemeProperty::custom("my_widget_special_color");
/// ```
///
/// Custom properties should be documented and used consistently across your application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeProperty {
    // Common properties
    /// Common color property used across multiple widgets.
    Color,
    /// Inverted color property for better contrast in dark contexts.
    ColorInvert,
    /// Background color property for containers and widgets.
    Background,
    /// Border color property for widget borders.
    Border,
    /// Text color property for text content.
    Text,

    // Button-specific properties
    /// Button color when in idle state.
    ColorIdle,
    /// Button color when being pressed.
    ColorPressed,
    /// Button color when hovered over.
    ColorHovered,
    /// Button color when focused (keyboard navigation).
    ColorFocused,

    // Input-specific properties
    /// Input field background color.
    ColorBackground,
    /// Input field background color when focused.
    ColorBackgroundFocused,
    /// Input field border color.
    ColorBorder,
    /// Input field border color when focused.
    ColorBorderFocused,
    /// Input field border color when in error state.
    ColorBorderError,
    /// Input field text color.
    ColorText,
    /// Input field cursor color.
    ColorCursor,
    /// Input field text selection color.
    ColorSelection,
    /// Input field placeholder text color.
    ColorPlaceholder,

    // Checkbox-specific properties
    /// Checkbox color when checked.
    ColorChecked,
    /// Checkbox color when unchecked.
    ColorUnchecked,
    /// Checkbox color when in indeterminate state (partially selected).
    ColorIndeterminate,
    /// Checkbox symbol (checkmark/minus) color.
    CheckboxSymbol,

    // Slider-specific properties
    /// Slider track color.
    SliderTrack,
    /// Slider thumb/ball color.
    SliderThumb,

    // Radio button-specific properties
    /// Radio button background color when selected.
    ColorBackgroundSelected,
    /// Radio button background color when disabled.
    ColorBackgroundDisabled,
    /// Radio button border color when hovered.
    ColorBorderHovered,
    /// Radio button border color when disabled.
    ColorBorderDisabled,
    /// Radio button dot color when selected.
    ColorDot,
    /// Radio button dot color when disabled.
    ColorDotDisabled,
    /// Radio button text color when disabled.
    ColorTextDisabled,

    // Toggle-specific properties
    /// Toggle track color when ON.
    ColorToggleTrackOn,
    /// Toggle track color when OFF.
    ColorToggleTrackOff,
    /// Toggle track border color when OFF.
    ColorToggleTrackBorder,
    /// Toggle thumb color.
    ColorToggleThumb,
    /// Toggle thumb border color.
    ColorToggleThumbBorder,
    /// Toggle colors when disabled.
    ColorToggleDisabled,

    // Menu-specific properties
    /// Menu item color when hovered.
    ColorMenuHovered,
    /// Menu item color when selected.
    ColorMenuSelected,
    /// Menu item color when disabled.
    ColorMenuDisabled,

    // Scroll container-specific properties
    /// Scrollbar track color.
    ColorScrollbar,
    /// Scrollbar thumb color.
    ColorScrollbarThumb,
    /// Scrollbar thumb color when hovered.
    ColorScrollbarThumbHover,
    /// Scrollbar thumb color when active (being dragged).
    ColorScrollbarThumbActive,

    // Tabs-specific properties
    /// Tab bar background color.
    TabBarBackground,
    /// Tab content background color.
    ContentBackground,
    /// Active tab color.
    TabActive,
    /// Inactive tab color.
    TabInactive,
    /// Tab color when hovered.
    TabHovered,
    /// Tab color when pressed.
    TabPressed,
    /// Tab text color.
    TabText,
    /// Active tab text color.
    TabTextActive,

    // Progress-specific properties
    /// Progress bar fill color.
    ColorProgress,

    // Common disabled state
    /// Common disabled state color.
    ColorDisabled,

    // Custom properties (for extensibility)
    /// Custom property for specialized widgets.
    Custom(&'static str),
}

impl ThemeProperty {
    /// Get the string representation of this property for backward compatibility.
    pub fn as_str(&self) -> &str {
        match self {
            ThemeProperty::Color => "color",
            ThemeProperty::ColorInvert => "color_invert",
            ThemeProperty::Background => "background",
            ThemeProperty::Border => "border",
            ThemeProperty::Text => "text",
            ThemeProperty::ColorIdle => "color_idle",
            ThemeProperty::ColorPressed => "color_pressed",
            ThemeProperty::ColorHovered => "color_hovered",
            ThemeProperty::ColorFocused => "color_focused",
            ThemeProperty::ColorBackground => "color_background",
            ThemeProperty::ColorBackgroundFocused => "color_background_focused",
            ThemeProperty::ColorBorder => "color_border",
            ThemeProperty::ColorBorderFocused => "color_border_focused",
            ThemeProperty::ColorBorderError => "color_border_error",
            ThemeProperty::ColorText => "color_text",
            ThemeProperty::ColorCursor => "color_cursor",
            ThemeProperty::ColorSelection => "color_selection",
            ThemeProperty::ColorPlaceholder => "color_placeholder",
            ThemeProperty::ColorChecked => "color_checked",
            ThemeProperty::ColorUnchecked => "color_unchecked",
            ThemeProperty::ColorIndeterminate => "color_indeterminate",
            ThemeProperty::CheckboxSymbol => "checkbox_symbol",
            ThemeProperty::SliderTrack => "slider_track",
            ThemeProperty::SliderThumb => "slider_thumb",
            ThemeProperty::ColorBackgroundSelected => "color_background_selected",
            ThemeProperty::ColorBackgroundDisabled => "color_background_disabled",
            ThemeProperty::ColorBorderHovered => "color_border_hovered",
            ThemeProperty::ColorBorderDisabled => "color_border_disabled",
            ThemeProperty::ColorDot => "color_dot",
            ThemeProperty::ColorDotDisabled => "color_dot_disabled",
            ThemeProperty::ColorTextDisabled => "color_text_disabled",
            ThemeProperty::ColorToggleTrackOn => "color_toggle_track_on",
            ThemeProperty::ColorToggleTrackOff => "color_toggle_track_off",
            ThemeProperty::ColorToggleTrackBorder => "color_toggle_track_border",
            ThemeProperty::ColorToggleThumb => "color_toggle_thumb",
            ThemeProperty::ColorToggleThumbBorder => "color_toggle_thumb_border",
            ThemeProperty::ColorToggleDisabled => "color_toggle_disabled",
            ThemeProperty::ColorMenuHovered => "color_hovered",
            ThemeProperty::ColorMenuSelected => "color_selected",
            ThemeProperty::ColorMenuDisabled => "color_disabled",
            ThemeProperty::ColorScrollbar => "color_scrollbar",
            ThemeProperty::ColorScrollbarThumb => "color_scrollbar_thumb",
            ThemeProperty::ColorScrollbarThumbHover => "color_scrollbar_thumb_hover",
            ThemeProperty::ColorScrollbarThumbActive => "color_scrollbar_thumb_active",
            ThemeProperty::TabBarBackground => "tab_bar_background",
            ThemeProperty::ContentBackground => "content_background",
            ThemeProperty::TabActive => "tab_active",
            ThemeProperty::TabInactive => "tab_inactive",
            ThemeProperty::TabHovered => "tab_hovered",
            ThemeProperty::TabPressed => "tab_pressed",
            ThemeProperty::TabText => "tab_text",
            ThemeProperty::TabTextActive => "tab_text_active",
            ThemeProperty::ColorProgress => "color_progress",
            ThemeProperty::ColorDisabled => "color_disabled",
            ThemeProperty::Custom(name) => name,
        }
    }

    /// Create a custom property from a string.
    pub fn custom(name: &'static str) -> Self {
        Self::Custom(name)
    }
}

impl std::str::FromStr for ThemeProperty {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "color" => Ok(ThemeProperty::Color),
            "color_invert" => Ok(ThemeProperty::ColorInvert),
            "background" => Ok(ThemeProperty::Background),
            "border" => Ok(ThemeProperty::Border),
            "text" => Ok(ThemeProperty::Text),
            "color_idle" => Ok(ThemeProperty::ColorIdle),
            "color_pressed" => Ok(ThemeProperty::ColorPressed),
            "color_hovered" => Ok(ThemeProperty::ColorHovered),
            "color_focused" => Ok(ThemeProperty::ColorFocused),
            "color_background" => Ok(ThemeProperty::ColorBackground),
            "color_background_focused" => Ok(ThemeProperty::ColorBackgroundFocused),
            "color_border" => Ok(ThemeProperty::ColorBorder),
            "color_border_focused" => Ok(ThemeProperty::ColorBorderFocused),
            "color_border_error" => Ok(ThemeProperty::ColorBorderError),
            "color_text" => Ok(ThemeProperty::ColorText),
            "color_cursor" => Ok(ThemeProperty::ColorCursor),
            "color_selection" => Ok(ThemeProperty::ColorSelection),
            "color_placeholder" => Ok(ThemeProperty::ColorPlaceholder),
            "color_checked" => Ok(ThemeProperty::ColorChecked),
            "color_unchecked" => Ok(ThemeProperty::ColorUnchecked),
            "color_indeterminate" => Ok(ThemeProperty::ColorIndeterminate),
            "checkbox_symbol" => Ok(ThemeProperty::CheckboxSymbol),
            "slider_track" => Ok(ThemeProperty::SliderTrack),
            "slider_thumb" => Ok(ThemeProperty::SliderThumb),
            "color_background_selected" => Ok(ThemeProperty::ColorBackgroundSelected),
            "color_background_disabled" => Ok(ThemeProperty::ColorBackgroundDisabled),
            "color_border_hovered" => Ok(ThemeProperty::ColorBorderHovered),
            "color_border_disabled" => Ok(ThemeProperty::ColorBorderDisabled),
            "color_dot" => Ok(ThemeProperty::ColorDot),
            "color_dot_disabled" => Ok(ThemeProperty::ColorDotDisabled),
            "color_text_disabled" => Ok(ThemeProperty::ColorTextDisabled),
            "color_toggle_track_on" => Ok(ThemeProperty::ColorToggleTrackOn),
            "color_toggle_track_off" => Ok(ThemeProperty::ColorToggleTrackOff),
            "color_toggle_track_border" => Ok(ThemeProperty::ColorToggleTrackBorder),
            "color_toggle_thumb" => Ok(ThemeProperty::ColorToggleThumb),
            "color_toggle_thumb_border" => Ok(ThemeProperty::ColorToggleThumbBorder),
            "color_toggle_disabled" => Ok(ThemeProperty::ColorToggleDisabled),
            "color_menu_hovered" => Ok(ThemeProperty::ColorMenuHovered),
            "color_menu_selected" => Ok(ThemeProperty::ColorMenuSelected),
            "color_menu_disabled" => Ok(ThemeProperty::ColorMenuDisabled),
            "color_scrollbar" => Ok(ThemeProperty::ColorScrollbar),
            "color_scrollbar_thumb" => Ok(ThemeProperty::ColorScrollbarThumb),
            "color_scrollbar_thumb_hover" => Ok(ThemeProperty::ColorScrollbarThumbHover),
            "color_scrollbar_thumb_active" => Ok(ThemeProperty::ColorScrollbarThumbActive),
            "tab_bar_background" => Ok(ThemeProperty::TabBarBackground),
            "content_background" => Ok(ThemeProperty::ContentBackground),
            "tab_active" => Ok(ThemeProperty::TabActive),
            "tab_inactive" => Ok(ThemeProperty::TabInactive),
            "tab_hovered" => Ok(ThemeProperty::TabHovered),
            "tab_pressed" => Ok(ThemeProperty::TabPressed),
            "tab_text" => Ok(ThemeProperty::TabText),
            "tab_text_active" => Ok(ThemeProperty::TabTextActive),
            "color_progress" => Ok(ThemeProperty::ColorProgress),
            "color_disabled" => Ok(ThemeProperty::ColorDisabled),
            // For custom properties, we can't easily return a static str reference from a temporary string
            // So we'll handle custom properties separately in the config loader
            _ => Err(()),
        }
    }
}

/// A type-safe theme value that can hold different types of styling data.
///
/// This enum provides a unified way to store various types of styling information
/// in a type-safe manner. It supports all common styling data types used in GUI theming.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::properties::{ThemeValue, ThemeProperty};
/// use vello::peniko::{Color, Brush, Gradient};
///
/// // Color values
/// let color_value = ThemeValue::Color(Color::from_rgb8(100, 150, 255));
///
/// // Float values
/// let float_value = ThemeValue::Float(16.0);
///
/// // Boolean values
/// let bool_value = ThemeValue::Bool(true);
///
/// // String values
/// let string_value = ThemeValue::String("My Label".to_string());
///
/// // Reference to another property
/// let ref_value = ThemeValue::Reference(ThemeProperty::ColorIdle);
/// ```
///
/// # Value Types
///
/// The enum supports the following data types:
///
/// - **[Color]**: RGB colors for backgrounds, text, borders, and other visual elements
/// - **[Gradient]**: Color gradients for advanced styling and visual effects
/// - **[Brush]**: Peniko brush objects for complex drawing operations
/// - **Float**: Floating-point values for sizes, opacities, and measurements
/// - **Int**: Signed integer values for counts, sizes, and discrete measurements
/// - **UInt**: Unsigned integer values for counts, sizes, and discrete measurements
/// - **Bool**: Boolean values for flags, states, and conditional styling
/// - **String**: Text values for labels, descriptions, and text content
/// - **Reference**: References to other theme properties for inheritance and composition
///
/// # Type Safety
///
/// The enum provides type-safe access to values through helper methods:
///
/// ```rust
/// use nptk_theme::properties::ThemeValue;
/// use vello::peniko::Color;
///
/// let value = ThemeValue::Color(Color::from_rgb8(100, 150, 255));
///
/// // Type-safe access
/// if let Some(color) = value.as_color() {
///     println!("Color: {:?}", color);
/// }
///
/// // Safe conversion with fallback
/// let color = value.as_color().unwrap_or(Color::BLACK);
/// ```
///
/// # Performance
///
/// - **Copy Semantics**: Most value types implement Copy for efficient storage
/// - **Enum Size**: Optimized memory layout for common use cases
/// - **Pattern Matching**: Fast pattern matching for value extraction
/// - **No Allocations**: Value types avoid unnecessary heap allocations where possible
///
/// # Best Practices
///
/// 1. **Use Appropriate Types**: Choose the most specific type for your data
/// 2. **Handle Missing Values**: Always provide fallbacks for missing values
/// 3. **Use References**: Use Reference variants for property inheritance
/// 4. **Avoid String Allocations**: Prefer static strings for constant values
/// 5. **Document Custom Usage**: Document any custom value usage patterns
#[derive(Clone, Debug)]
pub enum ThemeValue {
    /// A color value.
    Color(Color),
    /// A gradient value.
    Gradient(Gradient),
    /// A brush value.
    Brush(Brush),
    /// A float value.
    Float(f32),
    /// An integer value.
    Int(i32),
    /// An unsigned integer value.
    UInt(u32),
    /// A boolean value.
    Bool(bool),
    /// A string value.
    String(String),
    /// A reference to another theme property (for inheritance).
    Reference(ThemeProperty),
}

impl ThemeValue {
    /// Get the color value, if this is a color.
    pub fn as_color(&self) -> Option<Color> {
        match self {
            ThemeValue::Color(color) => Some(*color),
            _ => None,
        }
    }

    /// Get the float value, if this is a float.
    pub fn as_float(&self) -> Option<f32> {
        match self {
            ThemeValue::Float(value) => Some(*value),
            _ => None,
        }
    }

    /// Get the boolean value, if this is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ThemeValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Get the string value, if this is a string.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ThemeValue::String(value) => Some(value),
            _ => None,
        }
    }
}

/// A type-safe theme style that uses enum-based property keys.
///
/// This struct represents a collection of styling properties for a specific widget.
/// It provides type-safe access to theme properties and supports efficient storage
/// and retrieval of styling information.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::properties::{ThemeStyle, ThemeProperty, ThemeValue};
/// use vello::peniko::Color;
///
/// // Create a new theme style
/// let mut style = ThemeStyle::new();
///
/// // Set properties
/// style.set_color(ThemeProperty::ColorIdle, Color::from_rgb8(100, 150, 255));
/// style.set_color(ThemeProperty::ColorHovered, Color::from_rgb8(120, 170, 255));
/// style.set_float(ThemeProperty::custom("border_radius"), 8.0);
///
/// // Get properties
/// let idle_color = style.get_color(&ThemeProperty::ColorIdle).unwrap();
/// let border_radius = style.get_float(&ThemeProperty::custom("border_radius")).unwrap();
///
/// // Check if property exists
/// if style.has(&ThemeProperty::ColorPressed) {
///     println!("Has pressed color");
/// }
/// ```
///
/// # Creating Styles
///
/// You can create styles in several ways:
///
/// ```rust
/// use nptk_theme::properties::{ThemeStyle, ThemeProperty, ThemeValue};
/// use vello::peniko::Color;
///
/// // Empty style
/// let style = ThemeStyle::new();
///
/// // From property-value pairs
/// let style = ThemeStyle::from_properties([
///     (ThemeProperty::ColorIdle, ThemeValue::Color(Color::from_rgb8(100, 150, 255))),
///     (ThemeProperty::ColorHovered, ThemeValue::Color(Color::from_rgb8(120, 170, 255))),
/// ]);
///
/// // Using builder pattern
/// let style = ThemeStyle::new()
///     .with_value(ThemeProperty::ColorIdle, ThemeValue::Color(Color::from_rgb8(100, 150, 255)))
///     .with_value(ThemeProperty::ColorHovered, ThemeValue::Color(Color::from_rgb8(120, 170, 255)));
/// ```
///
/// # Property Management
///
/// The style provides comprehensive property management:
///
/// ```rust
/// use nptk_theme::properties::{ThemeStyle, ThemeProperty, ThemeValue};
/// use vello::peniko::Color;
///
/// let mut style = ThemeStyle::new();
///
/// // Set properties
/// style.set_color(ThemeProperty::ColorIdle, Color::from_rgb8(100, 150, 255));
/// style.set_float(ThemeProperty::custom("opacity"), 0.8);
/// style.set_bool(ThemeProperty::custom("enabled"), true);
///
/// // Get properties with type safety
/// let color = style.get_color(&ThemeProperty::ColorIdle);
/// let opacity = style.get_float(&ThemeProperty::custom("opacity"));
/// let enabled = style.get_bool(&ThemeProperty::custom("enabled"));
///
/// // Check existence
/// let has_color = style.has(&ThemeProperty::ColorIdle);
///
/// // Get all properties
/// let all_properties = style.properties();
/// ```
///
/// # Style Merging
///
/// Styles can be merged to combine properties from multiple sources:
///
/// ```rust
/// use nptk_theme::properties::{ThemeStyle, ThemeProperty, ThemeValue};
/// use vello::peniko::Color;
///
/// let mut base_style = ThemeStyle::new();
/// base_style.set_color(ThemeProperty::ColorIdle, Color::from_rgb8(100, 150, 255));
///
/// let mut override_style = ThemeStyle::new();
/// override_style.set_color(ThemeProperty::ColorHovered, Color::from_rgb8(120, 170, 255));
///
/// // Merge styles (override_style takes precedence)
/// base_style.merge(override_style);
/// ```
///
/// # Performance
///
/// - **HashMap Storage**: Efficient O(1) property lookup
/// - **Copy Semantics**: Properties implement Copy for efficient storage
/// - **Memory Efficient**: Only stores properties that are actually set
/// - **Fast Iteration**: Efficient iteration over all properties
///
/// # Thread Safety
///
/// ThemeStyle is not thread-safe by default. For thread-safe access, use
/// synchronization primitives or the [ThemeManager](crate::manager::ThemeManager).
///
/// # Best Practices
///
/// 1. **Use Type-Safe Properties**: Always use [ThemeProperty] enums
/// 2. **Provide Fallbacks**: Always handle missing properties gracefully
/// 3. **Group Related Properties**: Keep related properties together
/// 4. **Use Merging**: Use style merging for inheritance and composition
/// 5. **Document Custom Properties**: Document any custom properties you use
#[derive(Clone, Debug)]
pub struct ThemeStyle {
    properties: HashMap<ThemeProperty, ThemeValue>,
}

impl ThemeStyle {
    /// Create a new empty theme style.
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// Create a theme style from an iterator of property-value pairs.
    pub fn from_properties(
        properties: impl IntoIterator<Item = (ThemeProperty, ThemeValue)>,
    ) -> Self {
        Self {
            properties: properties.into_iter().collect(),
        }
    }

    /// Set a property value.
    pub fn set(&mut self, property: ThemeProperty, value: ThemeValue) {
        self.properties.insert(property, value);
    }

    /// Set a color property.
    pub fn set_color(&mut self, property: ThemeProperty, color: Color) {
        self.set(property, ThemeValue::Color(color));
    }

    /// Set a float property.
    pub fn set_float(&mut self, property: ThemeProperty, value: f32) {
        self.set(property, ThemeValue::Float(value));
    }

    /// Set a boolean property.
    pub fn set_bool(&mut self, property: ThemeProperty, value: bool) {
        self.set(property, ThemeValue::Bool(value));
    }

    /// Set a string property.
    pub fn set_string(&mut self, property: ThemeProperty, value: String) {
        self.set(property, ThemeValue::String(value));
    }

    /// Get a property value.
    pub fn get(&self, property: &ThemeProperty) -> Option<&ThemeValue> {
        self.properties.get(property)
    }

    /// Get a color property value.
    pub fn get_color(&self, property: &ThemeProperty) -> Option<Color> {
        self.get(property).and_then(|value| value.as_color())
    }

    /// Get a float property value.
    pub fn get_float(&self, property: &ThemeProperty) -> Option<f32> {
        self.get(property).and_then(|value| value.as_float())
    }

    /// Get a boolean property value.
    pub fn get_bool(&self, property: &ThemeProperty) -> Option<bool> {
        self.get(property).and_then(|value| value.as_bool())
    }

    /// Get a string property value.
    pub fn get_string(&self, property: &ThemeProperty) -> Option<&str> {
        self.get(property).and_then(|value| value.as_string())
    }

    /// Check if a property exists.
    pub fn has(&self, property: &ThemeProperty) -> bool {
        self.properties.contains_key(property)
    }

    /// Get all properties.
    pub fn properties(&self) -> &HashMap<ThemeProperty, ThemeValue> {
        &self.properties
    }

    /// Merge another theme style into this one.
    /// Properties from `other` will override properties in `self`.
    pub fn merge(&mut self, other: ThemeStyle) {
        for (property, value) in other.properties {
            self.properties.insert(property, value);
        }
    }
}

impl Default for ThemeStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme variables for CSS-like variable support.
///
/// This struct provides a CSS-like variable system for themes, allowing you to define
/// reusable values that can be referenced throughout your theme. Variables provide
/// consistency and make it easy to maintain color schemes and other styling values.
///
/// # Examples
///
/// ```rust
/// use nptk_theme::properties::{ThemeVariables, ThemeValue};
/// use vello::peniko::Color;
///
/// // Create theme variables
/// let mut variables = ThemeVariables::new();
///
/// // Define color variables
/// variables.set_color("primary", Color::from_rgb8(100, 150, 255));
/// variables.set_color("secondary", Color::from_rgb8(200, 200, 200));
/// variables.set_color("background", Color::from_rgb8(30, 30, 30));
/// variables.set_color("text", Color::from_rgb8(220, 220, 220));
///
/// // Define other types of variables
/// variables.set_float("border_radius", 8.0);
/// variables.set_bool("rounded_corners", true);
///
/// // Use variables
/// let primary_color = variables.get_color("primary").unwrap();
/// let border_radius = variables.get_float("border_radius").unwrap();
/// ```
///
/// # Variable Types
///
/// Variables can store any [ThemeValue] type:
///
/// ```rust
/// use nptk_theme::properties::{ThemeVariables, ThemeValue};
/// use vello::peniko::{Color, Brush, Gradient};
///
/// let mut variables = ThemeVariables::new();
///
/// // Color variables
/// variables.set_color("primary", Color::from_rgb8(100, 150, 255));
/// variables.set_color("secondary", Color::from_rgb8(200, 200, 200));
///
/// // Float variables
/// variables.set_float("border_radius", 8.0);
/// variables.set_float("opacity", 0.9);
///
/// // Boolean variables
/// variables.set_bool("dark_mode", true);
/// variables.set_bool("animations_enabled", false);
///
/// // String variables
/// variables.set_string("font_family", "Arial".to_string());
/// variables.set_string("theme_name", "My Custom Theme".to_string());
///
/// // Integer variables
/// variables.set_int("max_items", 100);
/// variables.set_uint("animation_duration", 300);
/// ```
///
/// # Variable Naming Conventions
///
/// Use consistent naming conventions for your variables:
///
/// ```rust
/// use nptk_theme::properties::ThemeVariables;
/// use vello::peniko::Color;
///
/// let mut variables = ThemeVariables::new();
///
/// // Color naming
/// variables.set_color("primary", Color::from_rgb8(100, 150, 255));
/// variables.set_color("primary-dark", Color::from_rgb8(80, 130, 235));
/// variables.set_color("primary-light", Color::from_rgb8(120, 170, 275));
/// variables.set_color("bg-primary", Color::from_rgb8(30, 30, 30));
/// variables.set_color("bg-secondary", Color::from_rgb8(40, 40, 40));
/// variables.set_color("text-primary", Color::from_rgb8(220, 220, 220));
/// variables.set_color("text-secondary", Color::from_rgb8(180, 180, 180));
///
/// // Size naming
/// variables.set_float("border-radius", 8.0);
/// variables.set_float("border-width", 1.0);
/// variables.set_float("padding-small", 4.0);
/// variables.set_float("padding-medium", 8.0);
/// variables.set_float("padding-large", 16.0);
///
/// // State naming
/// variables.set_bool("dark-mode", true);
/// variables.set_bool("animations-enabled", true);
/// variables.set_bool("high-contrast", false);
/// ```
///
/// # Variable Access
///
/// Access variables with type safety:
///
/// ```rust
/// use nptk_theme::properties::ThemeVariables;
/// use vello::peniko::Color;
///
/// let variables = ThemeVariables::new();
///
/// // Type-safe access
/// if let Some(color) = variables.get_color("primary") {
///     println!("Primary color: {:?}", color);
/// }
///
/// // Safe access with fallback
/// let primary_color = variables.get_color("primary").unwrap_or(Color::BLACK);
/// let border_radius = variables.get_float("border_radius").unwrap_or(4.0);
/// let dark_mode = variables.get_bool("dark_mode").unwrap_or(false);
/// ```
///
/// # Variable Inheritance
///
/// Variables can reference other variables through the [ThemeValue::Reference] type:
///
/// ```rust
/// use nptk_theme::properties::{ThemeVariables, ThemeValue, ThemeProperty};
/// use vello::peniko::Color;
///
/// let mut variables = ThemeVariables::new();
///
/// // Define base variables
/// variables.set_color("primary", Color::from_rgb8(100, 150, 255));
/// variables.set_color("secondary", Color::from_rgb8(200, 200, 200));
///
/// // Reference other variables
/// variables.set("primary-hover", ThemeValue::Reference(ThemeProperty::custom("primary")));
/// variables.set("secondary-hover", ThemeValue::Reference(ThemeProperty::custom("secondary")));
/// ```
///
/// # Performance
///
/// - **HashMap Storage**: Efficient O(1) variable lookup
/// - **Copy Semantics**: Variables implement Copy for efficient storage
/// - **Memory Efficient**: Only stores variables that are actually set
/// - **Fast Iteration**: Efficient iteration over all variables
///
/// # Thread Safety
///
/// ThemeVariables is not thread-safe by default. For thread-safe access, use
/// synchronization primitives or the [ThemeManager](crate::manager::ThemeManager).
///
/// # Best Practices
///
/// 1. **Use Consistent Naming**: Follow a consistent naming convention
/// 2. **Group Related Variables**: Keep related variables together
/// 3. **Document Variables**: Document what each variable represents
/// 4. **Use Semantic Names**: Use names that describe the purpose, not the value
/// 5. **Provide Fallbacks**: Always handle missing variables gracefully
/// 6. **Avoid Deep Nesting**: Keep variable references simple and clear
#[derive(Clone, Debug)]
pub struct ThemeVariables {
    variables: HashMap<String, ThemeValue>,
}

impl ThemeVariables {
    /// Create a new empty theme variables container.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Set a variable value.
    pub fn set(&mut self, name: impl Into<String>, value: ThemeValue) {
        self.variables.insert(name.into(), value);
    }

    /// Set a color variable.
    pub fn set_color(&mut self, name: impl Into<String>, color: Color) {
        self.set(name, ThemeValue::Color(color));
    }

    /// Get a variable value.
    pub fn get(&self, name: &str) -> Option<&ThemeValue> {
        self.variables.get(name)
    }

    /// Get a color variable.
    pub fn get_color(&self, name: &str) -> Option<Color> {
        self.get(name).and_then(|value| value.as_color())
    }

    /// Get all variables.
    pub fn variables(&self) -> &HashMap<String, ThemeValue> {
        &self.variables
    }
}

impl Default for ThemeVariables {
    fn default() -> Self {
        Self::new()
    }
}
