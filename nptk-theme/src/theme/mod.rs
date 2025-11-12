//! # Theme System
//!
//! This module provides the core theme system for NPTK, including the [Theme] trait
//! and built-in theme implementations. The theme system provides type-safe, efficient
//! access to widget styling information with support for runtime theme switching.
//!
//! ## Overview
//!
//! The theme system consists of:
//!
//! - **[Theme]**: The core trait that defines how themes work
//! - **[celeste::CelesteTheme]**: Light theme with cool blue-purple colors
//! - **[dark::DarkTheme]**: Dark theme with high contrast and modern styling
//! - **[sweet::SweetTheme]**: Modern dark theme with vibrant purple/magenta accents
//!
//! ## Key Features
//!
//! - **Type-Safe Access**: Enum-based properties instead of strings
//! - **Backward Compatibility**: Legacy string-based methods still supported
//! - **Fallback System**: Automatic fallbacks for missing properties
//! - **Variable Support**: CSS-like variables for consistent theming
//! - **Widget Support**: Check which widgets are supported
//! - **Extensibility**: Easy to implement custom themes
//!
//! ## Usage Examples
//!
//! ### Basic Theme Usage
//!
//! ```rust
//! use nptk_theme::theme::{Theme, celeste::CelesteTheme, dark::DarkTheme};
//! use nptk_theme::properties::ThemeProperty;
//! use nptk_theme::id::WidgetId;
//! use peniko::Color;
//!
//! // Create themes
//! let light_theme = CelesteTheme::light();
//! let dark_theme = DarkTheme::new();
//!
//! // Get theme properties
//! let button_id = WidgetId::new("nptk-widgets", "Button");
//! let idle_color = light_theme.get_property(button_id, &ThemeProperty::ColorIdle)
//!     .unwrap_or(Color::BLACK);
//! ```
//!
//! ### Theme Variables
//!
//! ```rust
//! use nptk_theme::theme::dark::DarkTheme;
//! use peniko::Color;
//!
//! let theme = DarkTheme::new();
//!
//! // Access theme variables
//! let primary_color = theme.variables().get_color("primary").unwrap();
//! let bg_color = theme.variables().get_color("bg-primary").unwrap();
//! ```
//!
//! ### Custom Theme Implementation
//!
//! ```rust
//! use nptk_theme::theme::Theme;
//! use nptk_theme::properties::{ThemeProperty, ThemeStyle, ThemeVariables};
//! use nptk_theme::id::WidgetId;
//! use nptk_theme::style::{DefaultStyles, Style, StyleVal};
//! use nptk_theme::globals::Globals;
//! use peniko::Color;
//!
//! struct MyCustomTheme {
//!     variables: ThemeVariables,
//!     globals: Globals,
//! }
//!
//! impl Theme for MyCustomTheme {
//!     fn of(&self, id: WidgetId) -> Option<Style> {
//!         // Implement legacy style access
//!         match id.namespace() {
//!             "nptk-widgets" => match id.id() {
//!                 "Button" => Some(Style::from_values([
//!                     ("color_idle".to_string(), StyleVal::Color(Color::from_rgb8(100, 150, 255))),
//!                     ("color_hovered".to_string(), StyleVal::Color(Color::from_rgb8(120, 170, 255))),
//!                 ])),
//!                 _ => None,
//!             },
//!             _ => None,
//!         }
//!     }
//!
//!     fn style(&self, id: WidgetId) -> Option<ThemeStyle> {
//!         // Implement type-safe style access
//!         match id.namespace() {
//!             "nptk-widgets" => match id.id() {
//!                 "Button" => {
//!                     let mut style = ThemeStyle::new();
//!                     style.set_color(ThemeProperty::ColorIdle, self.variables.get_color("primary").unwrap());
//!                     style.set_color(ThemeProperty::ColorHovered, self.variables.get_color("primary-light").unwrap());
//!                     Some(style)
//!                 },
//!                 _ => None,
//!             },
//!             _ => None,
//!         }
//!     }
//!
//!     fn defaults(&self) -> DefaultStyles {
//!         // Return default styles
//!         DefaultStyles::new(/* ... */)
//!     }
//!
//!     fn window_background(&self) -> Color {
//!         self.variables.get_color("bg-primary").unwrap_or(Color::WHITE)
//!     }
//!
//!     fn globals(&self) -> &Globals {
//!         &self.globals
//!     }
//!
//!     fn globals_mut(&mut self) -> &mut Globals {
//!         &mut self.globals
//!     }
//!
//!     fn variables(&self) -> ThemeVariables {
//!         self.variables.clone()
//!     }
//!
//!     fn variables_mut(&mut self) -> &mut ThemeVariables {
//!         &mut self.variables
//!     }
//!
//!     fn widget_id(&self) -> WidgetId {
//!         WidgetId::new("my-theme", "MyCustomTheme")
//!     }
//! }
//! ```
//!
//! ## Built-in Themes
//!
//! ### Celeste Theme (Light)
//!
//! A smooth and minimalistic light theme with cool blue and purple colors:
//!
//! ```rust
//! use nptk_theme::theme::celeste::CelesteTheme;
//!
//! let theme = CelesteTheme::light();
//! ```
//!
//! **Features:**
//! - Clean, modern appearance
//! - Cool blue-purple color scheme
//! - High contrast for readability
//! - Comprehensive widget support
//!
//! ### Dark Theme
//!
//! A modern dark theme with high contrast and excellent readability:
//!
//! ```rust
//! use nptk_theme::theme::dark::DarkTheme;
//!
//! let theme = DarkTheme::new();
//! ```
//!
//! **Features:**
//! - High contrast for low-light conditions
//! - Modern, professional appearance
//! - CSS-like variables for customization
//! - Comprehensive widget support
//!
//! ## Performance Considerations
//!
//! - **Caching**: Consider caching frequently accessed properties
//! - **Lazy Loading**: Load complex styles only when needed
//! - **Memory Usage**: Be mindful of memory usage for large themes
//! - **Thread Safety**: Ensure thread safety if used across threads
//!
//! ## Best Practices
//!
//! 1. **Use Type-Safe Methods**: Prefer `get_property()` over legacy `of()`
//! 2. **Provide Fallbacks**: Always provide sensible default values
//! 3. **Use Variables**: Define reusable values in `variables()`
//! 4. **Document Properties**: Document which properties your theme supports
//! 5. **Test Thoroughly**: Test all widget combinations with your theme

use peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeStyle, ThemeVariables};
use crate::rendering::ThemeRenderer;

/// The Celeste Theme.
pub mod celeste;
/// The Dark Theme.
pub mod dark;
/// The Sweet Theme.
pub mod sweet;
/// The System Theme.
pub mod system;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{celeste::CelesteTheme, dark::DarkTheme};

    #[test]
    fn test_theme_rendering_system() {
        // Test that built-in themes support rendering through supertrait
        let celeste_theme = CelesteTheme::light();
        let dark_theme = DarkTheme::new();

        // Both themes should support rendering (now automatic via supertrait)
        // We can test this by calling ThemeRenderer methods directly
        let button_id = WidgetId::new("nptk-widgets", "Button");
        let _color = celeste_theme
            .get_button_color(button_id.clone(), crate::rendering::WidgetState::Normal);
        let _color = dark_theme.get_button_color(button_id, crate::rendering::WidgetState::Normal);

        // Test that themes can be used as ThemeRenderer directly
        let _: &dyn ThemeRenderer = &celeste_theme;
        let _: &dyn ThemeRenderer = &dark_theme;
    }
}

/// Base trait for all themes in the NPTK theming system.
///
/// This trait defines the interface that all themes must implement. It provides
/// both legacy string-based access for backward compatibility and new type-safe
/// access methods for modern usage. All themes automatically support centralized
/// rendering through the ThemeRenderer supertrait.
///
/// # Key Features
///
/// - **Type-Safe Access**: New methods use enum-based properties
/// - **Backward Compatibility**: Legacy string-based methods still supported
/// - **Fallback System**: Automatic fallbacks for missing properties
/// - **Variable Support**: CSS-like variables for consistent theming
/// - **Widget Support**: Check which widgets are supported
/// - **Centralized Rendering**: All themes support centralized widget rendering
/// - **Extensibility**: Easy to implement custom themes
///
/// # Implementation Requirements
///
/// When implementing this trait, you must provide:
///
/// - `of()` - Legacy style access (for backward compatibility)
/// - `defaults()` - Default widget styles
/// - `window_background()` - Window background color
/// - `globals()` and `globals_mut()` - Global theme settings
/// - `widget_id()` - Unique identifier for the theme
///
/// # Optional Overrides
///
/// You can override these methods for enhanced functionality:
///
/// - `style()` - Type-safe style access (defaults to converting from legacy)
/// - `get_property()` - Direct property access with fallbacks
/// - `get_default_property()` - Default property values
/// - `variables()` and `variables_mut()` - CSS-like variables
/// - `supports_widget()` - Widget support checking
/// - `supported_widgets()` - List of supported widgets
/// - ThemeRenderer methods - Customize rendering behavior
///
/// # Usage
///
/// ```rust
/// use nptk_theme::theme::Theme;
/// use nptk_theme::properties::ThemeProperty;
/// use nptk_theme::id::WidgetId;
/// use peniko::Color;
///
/// // Type-safe property access (recommended)
/// let color = theme.get_property(
///     WidgetId::new("nptk-widgets", "Button"),
///     &ThemeProperty::ColorIdle
/// ).unwrap_or(Color::BLACK);
///
/// // Legacy access (still supported)
/// if let Some(style) = theme.of(WidgetId::new("nptk-widgets", "Button")) {
///     let color = style.get_color("color_idle").unwrap_or(Color::BLACK);
/// }
///
/// // Variable access
/// if let Some(primary_color) = theme.variables().get_color("primary") {
///     // Use primary color
/// }
/// ```
///
/// # Best Practices
///
/// 1. **Use Type-Safe Methods**: Prefer `get_property()` over legacy `of()`
/// 2. **Provide Fallbacks**: Always provide sensible default values
/// 3. **Use Variables**: Define reusable values in `variables()`
/// 4. **Document Properties**: Document which properties your theme supports
/// 5. **Test Thoroughly**: Test all widget combinations with your theme
///
/// # Performance Considerations
///
/// - **Caching**: Consider caching frequently accessed properties
/// - **Lazy Loading**: Load complex styles only when needed
/// - **Memory Usage**: Be mindful of memory usage for large themes
/// - **Thread Safety**: Ensure thread safety if used across threads
pub trait Theme: ThemeRenderer {
    /// Return the type-safe [ThemeStyle] of the given widget using its ID.
    /// Returns [None] if the theme does not have styles for the given widget.
    /// This is the preferred method for accessing theme properties.
    fn style(&self, _id: WidgetId) -> Option<ThemeStyle> {
        // Default implementation - themes should override this for better performance
        None
    }

    /// Get a specific theme property for a widget with fallback to defaults.
    /// This is the recommended way to access theme properties.
    fn get_property(&self, id: WidgetId, property: &ThemeProperty) -> Option<Color> {
        self.style(id)
            .and_then(|style| style.get_color(property))
            .or_else(|| self.get_default_property(property))
    }

    /// Get a default property value for when widget-specific styles are not available.
    fn get_default_property(&self, property: &ThemeProperty) -> Option<Color> {
        match property {
            ThemeProperty::Color | ThemeProperty::Text => Some(Color::from_rgb8(0, 0, 0)),
            ThemeProperty::ColorBackground | ThemeProperty::Background => {
                Some(Color::from_rgb8(255, 255, 255))
            },
            ThemeProperty::Border | ThemeProperty::ColorBorder => {
                Some(Color::from_rgb8(200, 200, 200))
            },
            ThemeProperty::ColorIdle => Some(Color::from_rgb8(200, 200, 200)),
            ThemeProperty::ColorHovered => Some(Color::from_rgb8(180, 180, 180)),
            ThemeProperty::ColorPressed => Some(Color::from_rgb8(160, 160, 160)),
            ThemeProperty::ColorDisabled => Some(Color::from_rgb8(150, 150, 150)),
            ThemeProperty::ColorMenuHovered => Some(Color::from_rgb8(220, 220, 220)),
            ThemeProperty::ColorMenuSelected => Some(Color::from_rgb8(100, 150, 255)),
            ThemeProperty::ColorMenuDisabled => Some(Color::from_rgb8(150, 150, 150)),
            ThemeProperty::CheckboxSymbol => Some(Color::from_rgb8(255, 255, 255)),
            _ => None,
        }
    }

    /// Get the background color of the window.
    fn window_background(&self) -> Color;

    /// Get global style values.
    fn globals(&self) -> &Globals;

    /// Get mutable global style values.
    fn globals_mut(&mut self) -> &mut Globals;

    /// Get theme variables for CSS-like variable support.
    fn variables(&self) -> ThemeVariables {
        // Default implementation returns empty variables
        // Note: This creates a new instance each time, themes should override this method
        ThemeVariables::new()
    }

    /// Get mutable theme variables.
    fn variables_mut(&mut self) -> &mut ThemeVariables {
        // Default implementation - themes should override this if they support variables
        // Note: This creates a new instance each time, themes should override this method
        Box::leak(Box::new(ThemeVariables::new()))
    }

    /// Check if this theme supports a specific widget.
    fn supports_widget(&self, id: WidgetId) -> bool {
        self.style(id).is_some()
    }

    /// Get all supported widget IDs.
    fn supported_widgets(&self) -> Vec<WidgetId> {
        // Default implementation - themes should override this for better performance
        vec![]
    }

    /// Get the widget ID for this theme (for identification purposes).
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-theme", "UnknownTheme")
    }

    /// Get a reference to this theme as Any for downcasting.
    fn as_any(&self) -> &dyn std::any::Any;
}
