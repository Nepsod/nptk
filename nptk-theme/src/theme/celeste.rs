use vello::peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::theme::{LayoutMetrics, ProvidesLayoutMetrics, ProvidesPalette, Theme, ThemePalette};

/// A smooth and minimalistic theme with a cold blue and purple touch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CelesteTheme {
    /// Use [CelesteTheme::light] to use the light Celeste theme.
    Light(CelesteThemeData),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Data for the Celeste theme.
pub struct CelesteThemeData {
    globals: Globals,
    palette: ThemePalette,
    metrics: LayoutMetrics,
}

impl CelesteTheme {
    /// The Light Celeste Theme.
    pub fn light() -> Self {
        let palette = ThemePalette::celeste_light();
        let metrics = LayoutMetrics::classic_light();
        let globals = Globals {
            invert_text_color: metrics.prefers_inverted_text,
            ..Globals::default()
        };
        Self::Light(CelesteThemeData {
            globals,
            palette,
            metrics,
        })
    }

    fn data(&self) -> &CelesteThemeData {
        match self {
            CelesteTheme::Light(data) => data,
        }
    }

    fn data_mut(&mut self) -> &mut CelesteThemeData {
        match self {
            CelesteTheme::Light(data) => data,
        }
    }
}

impl Default for CelesteTheme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme for CelesteTheme {
    fn get_property(
        &self,
        id: WidgetId,
        property: &crate::properties::ThemeProperty,
    ) -> Option<Color> {
        let palette = match self {
            CelesteTheme::Light(data) => &data.palette,
        };

        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => match property {
                    crate::properties::ThemeProperty::Color => Some(palette.text),
                    crate::properties::ThemeProperty::ColorInvert => Some(palette.background),
                    _ => None,
                },
                "Button" => match property {
                    crate::properties::ThemeProperty::ColorIdle => Some(palette.primary),
                    crate::properties::ThemeProperty::ColorPressed => Some(palette.primary_dark),
                    crate::properties::ThemeProperty::ColorHovered => Some(palette.primary_light),
                    crate::properties::ThemeProperty::ColorFocused => Some(palette.primary_dark),
                    _ => None,
                },
                "Checkbox" => match property {
                    crate::properties::ThemeProperty::ColorChecked
                    | crate::properties::ThemeProperty::ColorIndeterminate => {
                        Some(palette.primary_light)
                    },
                    crate::properties::ThemeProperty::ColorUnchecked => Some(palette.primary_light),
                    _ => None,
                },
                "Slider" => match property {
                    crate::properties::ThemeProperty::SliderTrack => Some(palette.primary_dark),
                    crate::properties::ThemeProperty::SliderThumb => Some(palette.primary_light),
                    _ => None,
                },
                "TextInput" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(palette.background),
                    crate::properties::ThemeProperty::ColorBorder => Some(palette.border),
                    crate::properties::ThemeProperty::ColorBorderFocused => Some(palette.accent),
                    crate::properties::ThemeProperty::ColorText => Some(palette.text),
                    _ => None,
                },
                "Progress" => match property {
                    crate::properties::ThemeProperty::Color => Some(palette.background_elevated),
                    crate::properties::ThemeProperty::ColorProgress => Some(palette.primary),
                    crate::properties::ThemeProperty::ColorBorder => Some(palette.border),
                    _ => None,
                },
                "MenuBar" => match property {
                    crate::properties::ThemeProperty::ColorBackground => {
                        Some(palette.background_alt)
                    },
                    crate::properties::ThemeProperty::ColorBorder => Some(palette.border),
                    crate::properties::ThemeProperty::ColorText => Some(palette.text),
                    crate::properties::ThemeProperty::ColorDisabled => Some(palette.text_muted),
                    crate::properties::ThemeProperty::ColorMenuSelected => Some(palette.selection),
                    crate::properties::ThemeProperty::ColorMenuHovered => {
                        Some(palette.background_elevated)
                    },
                    _ => None,
                },
                "MenuPopup" => match property {
                    crate::properties::ThemeProperty::ColorBackground => {
                        Some(palette.background_alt)
                    },
                    crate::properties::ThemeProperty::ColorBorder => Some(palette.border),
                    crate::properties::ThemeProperty::ColorText => Some(palette.text),
                    crate::properties::ThemeProperty::ColorDisabled => Some(palette.text_muted),
                    crate::properties::ThemeProperty::ColorMenuHovered => {
                        Some(palette.background_elevated)
                    },
                    crate::properties::ThemeProperty::ColorMenuDisabled => Some(palette.text_muted),
                    _ => None,
                },
                "Toggle" => match property {
                    crate::properties::ThemeProperty::ColorToggleTrackOn => {
                        Some(palette.primary_light)
                    },
                    crate::properties::ThemeProperty::ColorToggleTrackOff => {
                        Some(palette.background_elevated)
                    },
                    crate::properties::ThemeProperty::ColorToggleTrackBorder => {
                        Some(palette.border)
                    },
                    crate::properties::ThemeProperty::ColorToggleThumb => {
                        Some(Color::from_rgb8(255, 255, 255))
                    },
                    crate::properties::ThemeProperty::ColorToggleThumbBorder => {
                        Some(palette.border)
                    },
                    crate::properties::ThemeProperty::ColorToggleDisabled => {
                        Some(palette.text_muted)
                    },

                    _ => None,
                },
                "Toolbar" => match property {
                    crate::properties::ThemeProperty::ColorToolbarBackground => {
                        Some(palette.background_alt)
                    },
                    crate::properties::ThemeProperty::ColorToolbarBorder => Some(palette.border),
                    crate::properties::ThemeProperty::ColorToolbarSeparator => Some(palette.border),

                    _ => None,
                },
                "ToolbarButton" => match property {
                    crate::properties::ThemeProperty::ColorIdle => Some(Color::TRANSPARENT),
                    crate::properties::ThemeProperty::ColorPressed => Some(palette.primary_dark),
                    crate::properties::ThemeProperty::ColorHovered => Some(palette.primary_light),
                    crate::properties::ThemeProperty::ColorFocused => Some(palette.primary_dark),
                    _ => None,
                },
                "FileList" | "FileListContent" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(palette.background),
                    crate::properties::ThemeProperty::ColorText => Some(palette.text),
                    crate::properties::ThemeProperty::ColorBackgroundSelected => {
                        Some(palette.selection)
                    },
                    crate::properties::ThemeProperty::ColorMenuHovered => {
                        Some(palette.background_elevated)
                    },
                    _ => None,
                },
                "FileListProperties" => match property {
                    crate::properties::ThemeProperty::ColorText => Some(palette.text),
                    crate::properties::ThemeProperty::ColorTextDisabled => Some(palette.text_muted),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn window_background(&self) -> Color {
        self.data().palette.background
    }

    fn globals(&self) -> &Globals {
        &self.data().globals
    }

    fn globals_mut(&mut self) -> &mut Globals {
        &mut self.data_mut().globals
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-theme", "CelesteTheme")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    // ThemeRenderer methods are now inherited via supertrait
}

// ThemeRenderer is automatically implemented via blanket impl for all Theme types

impl ProvidesPalette for CelesteTheme {
    fn palette(&self) -> &ThemePalette {
        &self.data().palette
    }
}

impl ProvidesLayoutMetrics for CelesteTheme {
    fn layout_metrics(&self) -> LayoutMetrics {
        self.data().metrics
    }
}
