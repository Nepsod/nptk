use peniko::{Brush, Color, Gradient};
use std::collections::HashMap;

/// Type-safe theme property keys for widgets.
/// This eliminates string-based property access and provides compile-time safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeProperty {
    // Common properties
    Color,
    ColorInvert,
    Background,
    Border,
    Text,
    
    // Button-specific properties
    ColorIdle,
    ColorPressed,
    ColorHovered,
    ColorFocused,
    
    // Input-specific properties
    ColorBackground,
    ColorBackgroundFocused,
    ColorBorder,
    ColorBorderFocused,
    ColorBorderError,
    ColorText,
    ColorCursor,
    ColorSelection,
    ColorPlaceholder,
    
    // Checkbox-specific properties
    ColorChecked,
    ColorUnchecked,
    
    // Slider-specific properties
    ColorBall,
    
    // Radio button-specific properties
    ColorBackgroundSelected,
    ColorBackgroundDisabled,
    ColorBorderHovered,
    ColorBorderDisabled,
    ColorDot,
    ColorDotDisabled,
    ColorTextDisabled,
    
    // Menu-specific properties
    ColorMenuHovered,
    ColorMenuSelected,
    ColorMenuDisabled,
    
    // Scroll container-specific properties
    ColorScrollbar,
    ColorScrollbarThumb,
    ColorScrollbarThumbHover,
    ColorScrollbarThumbActive,
    
    // Tabs-specific properties
    TabBarBackground,
    ContentBackground,
    TabActive,
    TabInactive,
    TabHovered,
    TabPressed,
    TabText,
    TabTextActive,
    
    // Progress-specific properties
    ColorProgress,
    
    // Common disabled state
    ColorDisabled,
    
    // Custom properties (for extensibility)
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
            ThemeProperty::ColorBall => "color_ball",
            ThemeProperty::ColorBackgroundSelected => "color_background_selected",
            ThemeProperty::ColorBackgroundDisabled => "color_background_disabled",
            ThemeProperty::ColorBorderHovered => "color_border_hovered",
            ThemeProperty::ColorBorderDisabled => "color_border_disabled",
            ThemeProperty::ColorDot => "color_dot",
            ThemeProperty::ColorDotDisabled => "color_dot_disabled",
            ThemeProperty::ColorTextDisabled => "color_text_disabled",
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

/// A type-safe theme value that can hold different types of styling data.
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
    pub fn from_properties(properties: impl IntoIterator<Item = (ThemeProperty, ThemeValue)>) -> Self {
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
