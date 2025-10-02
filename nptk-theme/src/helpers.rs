use peniko::Color;
use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::Theme;

/// Helper functions for safe theme property access with proper fallbacks.
pub struct ThemeHelper;

impl ThemeHelper {
    /// Get a color property with safe fallbacks.
    /// This is the recommended way to access theme colors in widgets.
    pub fn get_color_safe<T: Theme>(
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
        
        // Use first fallback
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
            }
            InputColorProperty::Border => {
                if !is_valid {
                    ThemeProperty::ColorBorderError
                } else if is_focused {
                    ThemeProperty::ColorBorderFocused
                } else {
                    ThemeProperty::ColorBorder
                }
            }
            InputColorProperty::Text => ThemeProperty::ColorText,
            InputColorProperty::Cursor => ThemeProperty::ColorCursor,
            InputColorProperty::Selection => ThemeProperty::ColorSelection,
            InputColorProperty::Placeholder => ThemeProperty::ColorPlaceholder,
        };
        
        let fallback = match property {
            InputColorProperty::Background => {
                if is_focused { Color::WHITE } else { Color::from_rgb8(240, 240, 240) }
            }
            InputColorProperty::Border => {
                if !is_valid { Color::from_rgb8(255, 0, 0) }
                else if is_focused { Color::from_rgb8(0, 120, 255) }
                else { Color::from_rgb8(200, 200, 200) }
            }
            InputColorProperty::Text => Color::BLACK,
            InputColorProperty::Cursor => Color::BLACK,
            InputColorProperty::Selection => Color::from_rgb8(180, 200, 255),
            InputColorProperty::Placeholder => Color::from_rgb8(150, 150, 150),
        };
        
        Self::get_color_safe(theme, widget_id, &theme_property, fallback)
    }
    
    /// Get a checkbox color based on state with safe fallbacks.
    pub fn get_checkbox_color<T: Theme>(
        theme: &T,
        widget_id: WidgetId,
        is_checked: bool,
    ) -> Color {
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
    Idle,
    Hovered,
    Pressed,
    Released,
}

/// Input field color properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputColorProperty {
    Background,
    Border,
    Text,
    Cursor,
    Selection,
    Placeholder,
}

/// Progress bar color properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressColorProperty {
    Background,
    Progress,
    Border,
}
