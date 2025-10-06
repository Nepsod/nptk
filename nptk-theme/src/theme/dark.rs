use peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeStyle, ThemeVariables};
use crate::style::{
    DefaultContainerStyles, DefaultInteractiveStyles, DefaultStyles, DefaultTextStyles, Style,
    StyleVal,
};
use crate::theme::Theme;
use crate::rendering::ThemeRenderer;

/// A dark theme with high contrast and modern styling.
#[derive(Debug, Clone)]
pub struct DarkTheme {
    globals: Globals,
    variables: ThemeVariables,
}

impl DarkTheme {
    /// Create a new dark theme.
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
        self.variables.set_color("primary", Color::from_rgb8(100, 150, 255));
        self.variables.set_color("primary-dark", Color::from_rgb8(80, 130, 235));
        self.variables.set_color("primary-light", Color::from_rgb8(120, 170, 255));
        
        // Background colors
        self.variables.set_color("bg-primary", Color::from_rgb8(30, 30, 30));
        self.variables.set_color("bg-secondary", Color::from_rgb8(40, 40, 40));
        self.variables.set_color("bg-tertiary", Color::from_rgb8(50, 50, 50));
        
        // Text colors
        self.variables.set_color("text-primary", Color::from_rgb8(220, 220, 220));
        self.variables.set_color("text-secondary", Color::from_rgb8(180, 180, 180));
        self.variables.set_color("text-muted", Color::from_rgb8(140, 140, 140));
        
        // Border colors
        self.variables.set_color("border-primary", Color::from_rgb8(80, 80, 80));
        self.variables.set_color("border-secondary", Color::from_rgb8(100, 100, 100));
        
        // State colors
        self.variables.set_color("success", Color::from_rgb8(76, 175, 80));
        self.variables.set_color("warning", Color::from_rgb8(255, 193, 7));
        self.variables.set_color("error", Color::from_rgb8(244, 67, 54));
        self.variables.set_color("info", Color::from_rgb8(33, 150, 243));
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

impl Default for DarkTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme for DarkTheme {
    fn of(&self, id: WidgetId) -> Option<Style> {
        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => Some(Style::from_values([
                    ("color".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_invert".to_string(), StyleVal::Color(self.variables.get_color("bg-primary").unwrap())),
                ])),

                "Button" => Some(Style::from_values([
                    ("color_idle".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_pressed".to_string(), StyleVal::Color(self.variables.get_color("primary-dark").unwrap())),
                    ("color_hovered".to_string(), StyleVal::Color(self.variables.get_color("primary-light").unwrap())),
                    ("color_focused".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                ])),

                "Checkbox" => Some(Style::from_values([
                    ("color_checked".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_unchecked".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                ])),

                "Slider" => Some(Style::from_values([
                    ("color".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_ball".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                ])),

                "TextInput" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_border_focused".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_text".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_cursor".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_selection".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                ])),

                "SecretInput" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_border_focused".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_text".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_cursor".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_selection".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                ])),

                "ValueInput" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_background_focused".to_string(), StyleVal::Color(self.variables.get_color("bg-tertiary").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_border_focused".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_border_error".to_string(), StyleVal::Color(self.variables.get_color("error").unwrap())),
                    ("color_text".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_cursor".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_placeholder".to_string(), StyleVal::Color(self.variables.get_color("text-muted").unwrap())),
                ])),

                "RadioButton" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_background_selected".to_string(), StyleVal::Color(self.variables.get_color("bg-tertiary").unwrap())),
                    ("color_background_disabled".to_string(), StyleVal::Color(self.variables.get_color("bg-primary").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_border_hovered".to_string(), StyleVal::Color(self.variables.get_color("border-secondary").unwrap())),
                    ("color_border_focused".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_border_disabled".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_dot".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_dot_disabled".to_string(), StyleVal::Color(self.variables.get_color("text-muted").unwrap())),
                    ("color_text".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_text_disabled".to_string(), StyleVal::Color(self.variables.get_color("text-muted").unwrap())),
                ])),

                "MenuBar" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_text".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_hovered".to_string(), StyleVal::Color(self.variables.get_color("bg-tertiary").unwrap())),
                    ("color_selected".to_string(), StyleVal::Color(self.variables.get_color("primary-dark").unwrap())),
                    ("color_disabled".to_string(), StyleVal::Color(self.variables.get_color("text-muted").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                ])),

                "MenuPopup" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_text".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                    ("color_hovered".to_string(), StyleVal::Color(self.variables.get_color("primary-dark").unwrap())),
                    ("color_disabled".to_string(), StyleVal::Color(self.variables.get_color("text-muted").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                ])),

                "ScrollContainer" => Some(Style::from_values([
                    ("color_background".to_string(), StyleVal::Color(self.variables.get_color("bg-primary").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("color_scrollbar".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_scrollbar_thumb".to_string(), StyleVal::Color(self.variables.get_color("border-secondary").unwrap())),
                    ("color_scrollbar_thumb_hover".to_string(), StyleVal::Color(self.variables.get_color("text-muted").unwrap())),
                    ("color_scrollbar_thumb_active".to_string(), StyleVal::Color(self.variables.get_color("text-secondary").unwrap())),
                ])),

                "TabsContainer" => Some(Style::from_values([
                    ("background".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                    ("tab_bar_background".to_string(), StyleVal::Color(self.variables.get_color("bg-tertiary").unwrap())),
                    ("content_background".to_string(), StyleVal::Color(self.variables.get_color("bg-primary").unwrap())),
                    ("tab_active".to_string(), StyleVal::Color(self.variables.get_color("bg-primary").unwrap())),
                    ("tab_inactive".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("tab_hovered".to_string(), StyleVal::Color(self.variables.get_color("bg-tertiary").unwrap())),
                    ("tab_pressed".to_string(), StyleVal::Color(self.variables.get_color("bg-primary").unwrap())),
                    ("tab_text".to_string(), StyleVal::Color(self.variables.get_color("text-secondary").unwrap())),
                    ("tab_text_active".to_string(), StyleVal::Color(self.variables.get_color("text-primary").unwrap())),
                ])),

                "Progress" => Some(Style::from_values([
                    ("color".to_string(), StyleVal::Color(self.variables.get_color("bg-secondary").unwrap())),
                    ("color_progress".to_string(), StyleVal::Color(self.variables.get_color("primary").unwrap())),
                    ("color_border".to_string(), StyleVal::Color(self.variables.get_color("border-primary").unwrap())),
                ])),

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
                    (ThemeProperty::ColorHovered, self.variables.get_color("primary-light").unwrap()),
                    (ThemeProperty::ColorFocused, self.variables.get_color("primary").unwrap()),
                ])),

                "Checkbox" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorChecked, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::ColorUnchecked, self.variables.get_color("border-primary").unwrap()),
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
                    (ThemeProperty::Background, self.variables.get_color("bg-secondary").unwrap()),
                    (ThemeProperty::ColorProgress, self.variables.get_color("primary").unwrap()),
                    (ThemeProperty::Border, self.variables.get_color("border-primary").unwrap()),
                ])),

                "MenuPopup" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorBackground, self.variables.get_color("bg-secondary").unwrap()),
                    (ThemeProperty::ColorText, self.variables.get_color("text-primary").unwrap()),
                    (ThemeProperty::ColorMenuHovered, self.variables.get_color("primary-dark").unwrap()),
                    (ThemeProperty::ColorMenuDisabled, self.variables.get_color("text-muted").unwrap()),
                    (ThemeProperty::Border, self.variables.get_color("border-primary").unwrap()),
                ])),

                _ => None,
            },
            _ => None,
        }
    }

    fn defaults(&self) -> DefaultStyles {
        DefaultStyles::new(
            DefaultTextStyles::new(
                self.variables.get_color("text-primary").unwrap(),
                self.variables.get_color("bg-primary").unwrap(),
            ),
            DefaultContainerStyles::new(
                self.variables.get_color("text-secondary").unwrap(),
                self.variables.get_color("bg-secondary").unwrap(),
            ),
            DefaultInteractiveStyles::new(
                self.variables.get_color("primary-dark").unwrap(),
                self.variables.get_color("primary").unwrap(),
                self.variables.get_color("primary-light").unwrap(),
                self.variables.get_color("text-muted").unwrap(),
            ),
        )
    }

    fn window_background(&self) -> Color {
        self.variables.get_color("bg-primary").unwrap()
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
        WidgetId::new("nptk-theme", "DarkTheme")
    }
    
    // supports_rendering() uses default implementation (returns true)
    // as_renderer() needs to be implemented to provide the renderer
    fn as_renderer(&mut self) -> Option<&mut dyn ThemeRenderer> {
        Some(self)
    }
}

impl ThemeRenderer for DarkTheme {
    // Use default implementations from the trait
}
