use vello::peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeStyle, ThemeVariables};
use crate::theme::{LayoutMetrics, ProvidesLayoutMetrics, ProvidesPalette, Theme, ThemePalette};

/// A dark theme with high contrast and modern styling.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DarkTheme {
    globals: Globals,
    variables: ThemeVariables,
    palette: ThemePalette,
    metrics: LayoutMetrics,
}

impl DarkTheme {
    /// Create a new dark theme.
    pub fn new() -> Self {
        let palette = ThemePalette::dark();
        let metrics = LayoutMetrics::modern_dark();
        let globals = Globals {
            invert_text_color: false,
            ..Globals::default()
        };

        let mut theme = Self {
            globals,
            variables: ThemeVariables::new(),
            palette,
            metrics,
        };

        // Set up theme variables
        theme.setup_variables();
        theme
    }

    /// Set up CSS-like variables for the theme.
    fn setup_variables(&mut self) {
        self.variables.set_color("primary", self.palette.primary);
        self.variables
            .set_color("primary-dark", self.palette.primary_dark);
        self.variables
            .set_color("primary-light", self.palette.primary_light);

        self.variables
            .set_color("bg-primary", self.palette.background);
        self.variables
            .set_color("bg-secondary", self.palette.background_alt);
        self.variables
            .set_color("bg-tertiary", self.palette.background_elevated);

        self.variables.set_color("text-primary", self.palette.text);
        self.variables
            .set_color("text-secondary", Color::from_rgb8(180, 180, 180));
        self.variables
            .set_color("text-muted", self.palette.text_muted);

        self.variables
            .set_color("border-primary", self.palette.border);
        self.variables
            .set_color("border-secondary", Color::from_rgb8(100, 100, 100));

        self.variables
            .set_color("success", Color::from_rgb8(76, 175, 80));
        self.variables
            .set_color("warning", Color::from_rgb8(255, 193, 7));
        self.variables
            .set_color("error", Color::from_rgb8(244, 67, 54));
        self.variables
            .set_color("info", Color::from_rgb8(33, 150, 243));
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
    fn get_property(
        &self,
        id: WidgetId,
        property: &crate::properties::ThemeProperty,
    ) -> Option<Color> {
        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => match property {
                    crate::properties::ThemeProperty::Color => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::WHITE),
                    ),
                    crate::properties::ThemeProperty::ColorInvert => Some(
                        self.variables
                            .get_color("bg-primary")
                            .unwrap_or(Color::BLACK),
                    ),
                    _ => None,
                },
                "Button" => match property {
                    crate::properties::ThemeProperty::ColorIdle => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorPressed => Some(
                        self.variables
                            .get_color("primary-dark")
                            .unwrap_or(Color::from_rgb8(80, 130, 235)),
                    ),
                    crate::properties::ThemeProperty::ColorHovered => Some(
                        self.variables
                            .get_color("primary-light")
                            .unwrap_or(Color::from_rgb8(120, 170, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorFocused => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    _ => None,
                },
                "Checkbox" => match property {
                    crate::properties::ThemeProperty::ColorChecked => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorUnchecked => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    _ => None,
                },
                "Slider" => match property {
                    crate::properties::ThemeProperty::SliderTrack => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    crate::properties::ThemeProperty::SliderThumb => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    _ => None,
                },
                "TextInput" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(
                        self.variables
                            .get_color("bg-secondary")
                            .unwrap_or(Color::from_rgb8(40, 40, 40)),
                    ),
                    crate::properties::ThemeProperty::ColorBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    crate::properties::ThemeProperty::ColorBorderFocused => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorText => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::from_rgb8(220, 220, 220)),
                    ),
                    crate::properties::ThemeProperty::ColorCursor => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::from_rgb8(220, 220, 220)),
                    ),
                    crate::properties::ThemeProperty::ColorSelection => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    _ => None,
                },
                "Progress" => match property {
                    crate::properties::ThemeProperty::Color => Some(Color::from_rgb8(60, 60, 60)),
                    crate::properties::ThemeProperty::ColorProgress => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    _ => None,
                },
                "MenuBar" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(
                        self.variables
                            .get_color("bg-secondary")
                            .unwrap_or(Color::from_rgb8(40, 40, 40)),
                    ),
                    crate::properties::ThemeProperty::ColorBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    crate::properties::ThemeProperty::ColorText => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::from_rgb8(220, 220, 220)),
                    ),
                    crate::properties::ThemeProperty::ColorDisabled => Some(
                        self.variables
                            .get_color("text-muted")
                            .unwrap_or(Color::from_rgb8(140, 140, 140)),
                    ),
                    crate::properties::ThemeProperty::ColorMenuSelected => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(
                        self.variables
                            .get_color("bg-tertiary")
                            .unwrap_or(Color::from_rgb8(50, 50, 50)),
                    ),
                    _ => None,
                },
                "MenuPopup" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(
                        self.variables
                            .get_color("bg-secondary")
                            .unwrap_or(Color::from_rgb8(40, 40, 40)),
                    ),
                    crate::properties::ThemeProperty::ColorBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    crate::properties::ThemeProperty::ColorText => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::from_rgb8(220, 220, 220)),
                    ),
                    crate::properties::ThemeProperty::ColorDisabled => Some(
                        self.variables
                            .get_color("text-muted")
                            .unwrap_or(Color::from_rgb8(140, 140, 140)),
                    ),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(
                        self.variables
                            .get_color("primary-dark")
                            .unwrap_or(Color::from_rgb8(80, 130, 235)),
                    ),
                    crate::properties::ThemeProperty::ColorMenuDisabled => Some(
                        self.variables
                            .get_color("text-muted")
                            .unwrap_or(Color::from_rgb8(140, 140, 140)),
                    ),
                    _ => None,
                },
                "Toggle" => match property {
                    crate::properties::ThemeProperty::ColorToggleTrackOn => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorToggleTrackOff => {
                        Some(Color::from_rgb8(60, 60, 60))
                    },
                    crate::properties::ThemeProperty::ColorToggleTrackBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    crate::properties::ThemeProperty::ColorToggleThumb => {
                        Some(Color::from_rgb8(255, 255, 255))
                    },
                    crate::properties::ThemeProperty::ColorToggleThumbBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(150, 150, 150)),
                    ),
                    crate::properties::ThemeProperty::ColorToggleDisabled => Some(
                        self.variables
                            .get_color("text-muted")
                            .unwrap_or(Color::from_rgb8(140, 140, 140)),
                    ),

                    _ => None,
                },
                "Toolbar" => match property {
                    crate::properties::ThemeProperty::ColorToolbarBackground => Some(
                        self.variables
                            .get_color("bg-secondary")
                            .unwrap_or(Color::from_rgb8(40, 40, 40)),
                    ),
                    crate::properties::ThemeProperty::ColorToolbarBorder => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),
                    crate::properties::ThemeProperty::ColorToolbarSeparator => Some(
                        self.variables
                            .get_color("border-primary")
                            .unwrap_or(Color::from_rgb8(80, 80, 80)),
                    ),

                    _ => None,
                },
                "ToolbarButton" => match property {
                    crate::properties::ThemeProperty::ColorIdle => Some(Color::TRANSPARENT),
                    crate::properties::ThemeProperty::ColorPressed => Some(
                        self.variables
                            .get_color("primary-dark")
                            .unwrap_or(Color::from_rgb8(80, 130, 235)),
                    ),
                    crate::properties::ThemeProperty::ColorHovered => Some(
                        self.variables
                            .get_color("primary-light")
                            .unwrap_or(Color::from_rgb8(120, 170, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorFocused => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    _ => None,
                },
                "FileList" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(
                        self.variables
                            .get_color("bg-primary")
                            .unwrap_or(Color::from_rgb8(30, 30, 30)),
                    ),
                    _ => None,
                },
                "FileListContent" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(
                        self.variables
                            .get_color("bg-primary")
                            .unwrap_or(Color::from_rgb8(30, 30, 30)),
                    ),
                    crate::properties::ThemeProperty::ColorText => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::from_rgb8(220, 220, 220)),
                    ),
                    crate::properties::ThemeProperty::ColorBackgroundSelected => Some(
                        self.variables
                            .get_color("primary")
                            .unwrap_or(Color::from_rgb8(100, 150, 255)),
                    ),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(
                        self.variables
                            .get_color("bg-tertiary")
                            .unwrap_or(Color::from_rgb8(50, 50, 50)),
                    ),
                    _ => None,
                },
                "FileListProperties" => match property {
                    crate::properties::ThemeProperty::ColorText => Some(
                        self.variables
                            .get_color("text-primary")
                            .unwrap_or(Color::from_rgb8(220, 220, 220)),
                    ),
                    crate::properties::ThemeProperty::ColorTextDisabled => Some(
                        self.variables
                            .get_color("text-muted")
                            .unwrap_or(Color::from_rgb8(140, 140, 140)),
                    ),
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
                    (
                        ThemeProperty::Color,
                        self.variables.get_color("text-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorInvert,
                        self.variables.get_color("bg-primary").unwrap(),
                    ),
                ])),

                "Button" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorIdle,
                        self.variables.get_color("primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorPressed,
                        self.variables.get_color("primary-dark").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorHovered,
                        self.variables.get_color("primary-light").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorFocused,
                        self.variables.get_color("primary").unwrap(),
                    ),
                ])),

                "Checkbox" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorChecked,
                        self.variables.get_color("primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorUnchecked,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                ])),

                "Slider" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::SliderTrack,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::SliderThumb,
                        self.variables.get_color("primary").unwrap(),
                    ),
                ])),

                "TextInput" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorBackground,
                        self.variables.get_color("bg-secondary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorBorder,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorBorderFocused,
                        self.variables.get_color("primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorText,
                        self.variables.get_color("text-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorCursor,
                        self.variables.get_color("text-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorSelection,
                        self.variables.get_color("primary").unwrap(),
                    ),
                ])),

                "Progress" => Some(self.create_widget_style(&[
                    (ThemeProperty::Color, Color::from_rgb8(60, 60, 60)),
                    (
                        ThemeProperty::ColorProgress,
                        self.variables.get_color("primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorBorder,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                ])),

                "MenuBar" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorBackground,
                        self.variables.get_color("bg-secondary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorBorder,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorText,
                        self.variables.get_color("text-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorDisabled,
                        self.variables.get_color("text-muted").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorMenuSelected,
                        self.variables.get_color("primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorMenuHovered,
                        self.variables.get_color("bg-tertiary").unwrap(),
                    ),
                ])),

                "MenuPopup" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorBackground,
                        self.variables.get_color("bg-secondary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorText,
                        self.variables.get_color("text-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorMenuHovered,
                        self.variables.get_color("primary-dark").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorMenuDisabled,
                        self.variables.get_color("text-muted").unwrap(),
                    ),
                    (
                        ThemeProperty::Border,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                ])),

                "Toggle" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorToggleTrackOn,
                        self.variables.get_color("primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorToggleTrackOff,
                        Color::from_rgb8(60, 60, 60),
                    ),
                    (
                        ThemeProperty::ColorToggleTrackBorder,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorToggleThumb,
                        Color::from_rgb8(255, 255, 255),
                    ),
                    (
                        ThemeProperty::ColorToggleThumbBorder,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorToggleDisabled,
                        self.variables.get_color("text-muted").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorToggleDisabled,
                        self.variables.get_color("text-muted").unwrap(),
                    ),
                ])),

                "Toolbar" => Some(self.create_widget_style(&[
                    (
                        ThemeProperty::ColorToolbarBackground,
                        self.variables.get_color("bg-secondary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorToolbarBorder,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorToolbarSeparator,
                        self.variables.get_color("border-primary").unwrap(),
                    ),
                ])),

                "ToolbarButton" => Some(self.create_widget_style(&[
                    (ThemeProperty::ColorIdle, Color::TRANSPARENT),
                    (
                        ThemeProperty::ColorPressed,
                        self.variables.get_color("primary-dark").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorHovered,
                        self.variables.get_color("primary-light").unwrap(),
                    ),
                    (
                        ThemeProperty::ColorFocused,
                        self.variables.get_color("primary").unwrap(),
                    ),
                ])),

                _ => None,
            },
            _ => None,
        }
    }

    fn window_background(&self) -> Color {
        self.variables
            .get_color("bg-primary")
            .unwrap_or(Color::from_rgb8(30, 30, 30))
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    // ThemeRenderer methods are now inherited via supertrait
}

// ThemeRenderer is automatically implemented via blanket impl for all Theme types

impl ProvidesPalette for DarkTheme {
    fn palette(&self) -> &ThemePalette {
        &self.palette
    }
}

impl ProvidesLayoutMetrics for DarkTheme {
    fn layout_metrics(&self) -> LayoutMetrics {
        self.metrics
    }
}
