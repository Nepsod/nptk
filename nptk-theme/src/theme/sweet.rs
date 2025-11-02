use peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeStyle, ThemeVariables};
use crate::theme::Theme;

/// A modern dark theme inspired by the Sweet Dark theme for Kvantum and KDE.
/// 
/// Features vibrant purple/magenta accents on a dark background with excellent contrast.
#[derive(Debug, Clone)]
pub struct SweetTheme {
    globals: Globals,
    variables: ThemeVariables,
}

impl SweetTheme {
    /// Create a new Sweet theme.
    pub fn new() -> Self {
        let mut theme = Self {
            globals: Globals::default(),
            variables: ThemeVariables::new(),
        };
        
        // Set up theme variables
        theme.setup_variables();
        theme
    }
    
    /// Set up CSS-like variables for the theme.
    fn setup_variables(&mut self) {
        // Primary colors
        self.variables.set_color("primary", Color::from_rgb8(197, 14, 210)); // controlAccentColor / highlight
        self.variables.set_color("primary-dark", Color::from_rgb8(157, 51, 213)); // systemPurpleColor
        self.variables.set_color("primary-light", Color::from_rgb8(254, 207, 14)); // systemYellowColor
        
        // Background colors - Sweet palette
        self.variables.set_color("bg-primary", Color::from_rgb8(22, 25, 37)); // backgroundColor2
        self.variables.set_color("bg-secondary", Color::from_rgb8(30, 34, 51)); // controlBackgroundColor
        self.variables.set_color("bg-tertiary", Color::from_rgb8(24, 27, 40)); // windowBackgroundColor
        self.variables.set_color("bg-quaternary", Color::from_rgb8(12, 14, 21)); // light.color / dark.color
        
        // Text colors
        self.variables.set_color("text-primary", Color::from_rgb8(211, 218, 227)); // textColor
        self.variables.set_color("text-secondary", Color::from_rgb8(195, 199, 209)); // labelColor
        self.variables.set_color("text-muted", Color::from_rgb8(102, 106, 115)); // secondaryLabelColor
        self.variables.set_color("text-disabled", Color::from_rgb8(102, 106, 115)); // disabled, with opacity
        
        // Border colors
        self.variables.set_color("border-primary", Color::from_rgb8(102, 106, 115)); // separatorColor
        self.variables.set_color("border-secondary", Color::from_rgb8(102, 106, 115)); // controlColor
        
        // Accent colors
        self.variables.set_color("accent-original", Color::from_rgb8(0, 232, 198)); // origControlAccentColor (green)
        self.variables.set_color("accent", Color::from_rgb8(197, 14, 210)); // controlAccentColor (magenta)
        
        // System colors
        self.variables.set_color("success", Color::from_rgb8(48, 211, 58)); // systemGreenColor
        self.variables.set_color("warning", Color::from_rgb8(254, 207, 14)); // systemYellowColor
        self.variables.set_color("error", Color::from_rgb8(251, 43, 44)); // systemRedColor
        self.variables.set_color("info", Color::from_rgb8(16, 106, 254)); // systemBlueColor
        self.variables.set_color("link", Color::from_rgb8(82, 148, 226)); // linkColor
        
        // Selection colors
        self.variables.set_color("selection-bg", Color::from_rgb8(197, 14, 210)); // selectedTextBackgroundColor
        self.variables.set_color("selection-text", Color::from_rgb8(254, 254, 254)); // selectedTextColor
        self.variables.set_color("selection-unfocused-bg", Color::from_rgb8(47, 52, 63)); // unemphasizedSelectedTextBackgroundColor
    }
    
    /// Create a type-safe theme style for a widget.
    fn create_widget_style(&self, properties: &[(ThemeProperty, Color)]) -> ThemeStyle {
        let mut style = ThemeStyle::new();
        for (property, color) in properties {
            style.set_color(*property, *color);
        }
        style
    }
}

impl Default for SweetTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme for SweetTheme {
    fn get_property(&self, id: WidgetId, property: &crate::properties::ThemeProperty) -> Option<Color> {
        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => match property {
                    crate::properties::ThemeProperty::Color => Some(self.variables.get_color("text-primary").unwrap_or(Color::from_rgb8(211, 218, 227))),
                    crate::properties::ThemeProperty::ColorInvert => Some(self.variables.get_color("bg-primary").unwrap_or(Color::from_rgb8(22, 25, 37))),
                    _ => None,
                },
                "Button" => match property {
                    crate::properties::ThemeProperty::ColorIdle => Some(self.variables.get_color("primary").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    crate::properties::ThemeProperty::ColorPressed => Some(self.variables.get_color("primary-dark").unwrap_or(Color::from_rgb8(157, 51, 213))),
                    crate::properties::ThemeProperty::ColorHovered => Some(self.variables.get_color("bg-tertiary").unwrap_or(Color::from_rgb8(24, 27, 40))),
                    crate::properties::ThemeProperty::ColorFocused => Some(self.variables.get_color("primary").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    _ => None,
                },
                "Checkbox" => match property {
                    crate::properties::ThemeProperty::ColorChecked => Some(self.variables.get_color("accent-original").unwrap_or(Color::from_rgb8(0, 232, 198))),
                    crate::properties::ThemeProperty::ColorUnchecked => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorIndeterminate => Some(self.variables.get_color("accent-original").unwrap_or(Color::from_rgb8(0, 232, 198))),
                    crate::properties::ThemeProperty::CheckboxSymbol => Some(Color::from_rgb8(0, 0, 0)),
                    _ => None,
                },
                "RadioButton" => match property {
                    // Background colors - using checkbox colors as base
                    crate::properties::ThemeProperty::ColorBackgroundSelected => Some(self.variables.get_color("accent-original").unwrap_or(Color::from_rgb8(0, 232, 198))),
                    crate::properties::ThemeProperty::ColorBackgroundDisabled => Some(self.variables.get_color("bg-tertiary").unwrap_or(Color::from_rgb8(24, 27, 40))),
                    crate::properties::ThemeProperty::ColorBackground => Some(self.variables.get_color("bg-secondary").unwrap_or(Color::from_rgb8(30, 34, 51))),
                    // Border colors - using checkbox border color as base
                    crate::properties::ThemeProperty::ColorBorder => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorBorderHovered => Some(self.variables.get_color("accent-original").unwrap_or(Color::from_rgb8(0, 232, 198))),
                    crate::properties::ThemeProperty::ColorBorderFocused => Some(self.variables.get_color("accent-original").unwrap_or(Color::from_rgb8(0, 232, 198))),
                    crate::properties::ThemeProperty::ColorBorderDisabled => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    // Dot colors - using checkbox symbol color as base
                    crate::properties::ThemeProperty::ColorDot => Some(Color::from_rgb8(0, 0, 0)),
                    crate::properties::ThemeProperty::ColorDotDisabled => Some(self.variables.get_color("text-muted").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    // Text colors
                    crate::properties::ThemeProperty::ColorText => Some(self.variables.get_color("text-primary").unwrap_or(Color::from_rgb8(211, 218, 227))),
                    crate::properties::ThemeProperty::ColorTextDisabled => Some(self.variables.get_color("text-muted").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    _ => None,
                },
                "Slider" => match property {
                    crate::properties::ThemeProperty::Color => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorBall => Some(self.variables.get_color("primary").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    _ => None,
                },
                "TextInput" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(self.variables.get_color("bg-secondary").unwrap_or(Color::from_rgb8(30, 34, 51))),
                    crate::properties::ThemeProperty::ColorBorder => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorBorderFocused => Some(self.variables.get_color("primary").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    crate::properties::ThemeProperty::ColorText => Some(self.variables.get_color("text-primary").unwrap_or(Color::from_rgb8(211, 218, 227))),
                    crate::properties::ThemeProperty::ColorCursor => Some(self.variables.get_color("text-primary").unwrap_or(Color::from_rgb8(211, 218, 227))),
                    crate::properties::ThemeProperty::ColorSelection => Some(self.variables.get_color("primary").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    _ => None,
                },
                "Progress" => match property {
                    crate::properties::ThemeProperty::Color => Some(Color::from_rgb8(30, 34, 51)),
                    crate::properties::ThemeProperty::ColorProgress => Some(self.variables.get_color("primary").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    crate::properties::ThemeProperty::ColorBorder => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    _ => None,
                },
                "MenuBar" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(self.variables.get_color("bg-tertiary").unwrap_or(Color::from_rgb8(24, 27, 40))),
                    crate::properties::ThemeProperty::ColorBorder => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorText => Some(self.variables.get_color("text-secondary").unwrap_or(Color::from_rgb8(195, 199, 209))),
                    crate::properties::ThemeProperty::ColorDisabled => Some(self.variables.get_color("text-muted").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorMenuSelected => Some(self.variables.get_color("accent").unwrap_or(Color::from_rgb8(197, 14, 210))),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(self.variables.get_color("primary-dark").unwrap_or(Color::from_rgb8(157, 51, 213))),
                    _ => None,
                },
                "MenuPopup" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(self.variables.get_color("bg-secondary").unwrap_or(Color::from_rgb8(30, 34, 51))),
                    crate::properties::ThemeProperty::ColorBorder => Some(self.variables.get_color("border-primary").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorText => Some(self.variables.get_color("text-secondary").unwrap_or(Color::from_rgb8(195, 199, 209))),
                    crate::properties::ThemeProperty::ColorDisabled => Some(self.variables.get_color("text-muted").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(self.variables.get_color("primary-dark").unwrap_or(Color::from_rgb8(157, 51, 213))),
                    crate::properties::ThemeProperty::ColorMenuDisabled => Some(self.variables.get_color("text-muted").unwrap_or(Color::from_rgb8(102, 106, 115))),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn style(&self, id: WidgetId) -> Option<ThemeStyle> {
        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => Some(self.create_widget_style(&[
                    (ThemeProperty::Color, self.variables.get_color("text-primary").unwrap()),
                    (ThemeProperty::ColorInvert, self.variables.get_color("bg-primary").unwrap()),
                ])),

                "Button" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorIdle, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorPressed, self.variables.get_color("primary-dark").unwrap()),
                    (ThemeProperty::ColorHovered, self.variables.get_color("bg-tertiary").unwrap()),
                    (ThemeProperty::ColorFocused, self.variables.get_color("primary").unwrap()),
                ])),

                "Checkbox" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorChecked, self.variables.get_color("accent-original").unwrap()),
                    (ThemeProperty::ColorUnchecked, self.variables.get_color("border-primary").unwrap()),
                    (ThemeProperty::ColorIndeterminate, self.variables.get_color("accent-original").unwrap()),
                    (ThemeProperty::CheckboxSymbol, Color::from_rgb8(0, 0, 0)),
                ])),

                "RadioButton" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorBackgroundSelected, self.variables.get_color("accent-original").unwrap()),
                    (ThemeProperty::ColorBackgroundDisabled, self.variables.get_color("bg-tertiary").unwrap()),
                    (ThemeProperty::ColorBackground, self.variables.get_color("bg-secondary").unwrap()),
                    (ThemeProperty::ColorBorder, self.variables.get_color("border-primary").unwrap()),
                    (ThemeProperty::ColorBorderHovered, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorBorderFocused, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorBorderDisabled, self.variables.get_color("border-primary").unwrap()),
                    (ThemeProperty::ColorDot, Color::from_rgb8(0, 0, 0)),
                    (ThemeProperty::ColorDotDisabled, self.variables.get_color("text-muted").unwrap()),
                    (ThemeProperty::ColorText, self.variables.get_color("text-primary").unwrap()),
                    (ThemeProperty::ColorTextDisabled, self.variables.get_color("text-muted").unwrap()),
                ])),

                "Slider" => Some(self.create_widget_style(&[
                    (ThemeProperty::Color, self.variables.get_color("border-primary").unwrap()),
                    (ThemeProperty::ColorBall, self.variables.get_color("primary").unwrap()),
                ])),

                "TextInput" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorBackground, self.variables.get_color("bg-secondary").unwrap()),
                    (ThemeProperty::ColorBorder, self.variables.get_color("border-primary").unwrap()),
                    (ThemeProperty::ColorBorderFocused, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorText, self.variables.get_color("text-primary").unwrap()),
                    (ThemeProperty::ColorCursor, self.variables.get_color("text-primary").unwrap()),
                    (ThemeProperty::ColorSelection, self.variables.get_color("primary").unwrap()),
                ])),

                "Progress" => Some(self.create_widget_style(&[
                    (ThemeProperty::Color, Color::from_rgb8(30, 34, 51)),
                    (ThemeProperty::ColorProgress, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorBorder, self.variables.get_color("border-primary").unwrap()),
                ])),

                "MenuBar" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorBackground, self.variables.get_color("bg-tertiary").unwrap()),
                    (ThemeProperty::ColorBorder, self.variables.get_color("border-primary").unwrap()),
                    (ThemeProperty::ColorText, self.variables.get_color("text-secondary").unwrap()),
                    (ThemeProperty::ColorDisabled, self.variables.get_color("text-muted").unwrap()),
                    (ThemeProperty::ColorMenuSelected, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorMenuHovered, self.variables.get_color("bg-secondary").unwrap()),
                ])),

                "MenuPopup" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorBackground, self.variables.get_color("bg-secondary").unwrap()),
                    (ThemeProperty::ColorText, self.variables.get_color("text-secondary").unwrap()),
                    (ThemeProperty::ColorMenuHovered, self.variables.get_color("primary-dark").unwrap()),
                    (ThemeProperty::ColorMenuDisabled, self.variables.get_color("text-muted").unwrap()),
                    (ThemeProperty::Border, self.variables.get_color("border-primary").unwrap()),
                ])),

                _ => None,
            },
            _ => None,
        }
    }


    fn window_background(&self) -> Color {
        self.variables.get_color("bg-tertiary").unwrap_or(Color::from_rgb8(24, 27, 40))
    }

    fn globals(&self) -> &Globals {
        &self.globals
    }

    fn globals_mut(&mut self) -> &mut Globals {
        &mut self.globals
    }
    
    fn variables(&self) -> ThemeVariables {
        self.variables.clone()
    }
    
    fn variables_mut(&mut self) -> &mut ThemeVariables {
        &mut self.variables
    }
    
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-theme", "SweetTheme")
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    // ThemeRenderer methods are now inherited via supertrait
}

// ThemeRenderer is automatically implemented via blanket impl for all Theme types

