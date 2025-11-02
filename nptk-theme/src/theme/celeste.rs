use peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::theme::Theme;

/// A smooth and minimalistic theme with a cold blue and purple touch.
#[derive(Debug, Clone)]
pub enum CelesteTheme {
    /// Use [CelesteTheme::light] to use the light Celeste theme.
    Light(Globals),
}

impl CelesteTheme {
    /// The Light Celeste Theme.
    pub fn light() -> Self {
        Self::Light(Globals::default())
    }
}

impl Default for CelesteTheme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme for CelesteTheme {
    fn get_property(&self, id: WidgetId, property: &crate::properties::ThemeProperty) -> Option<Color> {
        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => match property {
                    crate::properties::ThemeProperty::Color => Some(Color::from_rgb8(0, 0, 0)),
                    crate::properties::ThemeProperty::ColorInvert => Some(Color::from_rgb8(255, 255, 255)),
                    _ => None,
                },
                "Button" => match property {
                    crate::properties::ThemeProperty::ColorIdle => Some(Color::from_rgb8(150, 170, 250)),
                    crate::properties::ThemeProperty::ColorPressed => Some(Color::from_rgb8(130, 150, 230)),
                    crate::properties::ThemeProperty::ColorHovered => Some(Color::from_rgb8(140, 160, 240)),
                    crate::properties::ThemeProperty::ColorFocused => Some(Color::from_rgb8(120, 140, 220)),
                    _ => None,
                },
                "Checkbox" => match property {
                    crate::properties::ThemeProperty::ColorChecked => Some(Color::from_rgb8(130, 130, 230)),
                    crate::properties::ThemeProperty::ColorUnchecked => Some(Color::from_rgb8(170, 170, 250)),
                    _ => None,
                },
                "Slider" => match property {
                    crate::properties::ThemeProperty::Color => Some(Color::from_rgb8(130, 130, 230)),
                    crate::properties::ThemeProperty::ColorBall => Some(Color::from_rgb8(170, 170, 250)),
                    _ => None,
                },
                "TextInput" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(Color::WHITE),
                    crate::properties::ThemeProperty::ColorBorder => Some(Color::from_rgb8(200, 200, 200)),
                    crate::properties::ThemeProperty::ColorBorderFocused => Some(Color::from_rgb8(100, 150, 255)),
                    crate::properties::ThemeProperty::ColorText => Some(Color::BLACK),
                    _ => None,
                },
                "Progress" => match property {
                    crate::properties::ThemeProperty::Color => Some(Color::from_rgb8(220, 220, 220)),
                    crate::properties::ThemeProperty::ColorProgress => Some(Color::from_rgb8(150, 170, 250)),
                    crate::properties::ThemeProperty::ColorBorder => Some(Color::from_rgb8(200, 200, 200)),
                    _ => None,
                },
                "MenuBar" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(Color::from_rgb8(245, 245, 245)),
                    crate::properties::ThemeProperty::ColorBorder => Some(Color::from_rgb8(200, 200, 200)),
                    crate::properties::ThemeProperty::ColorText => Some(Color::from_rgb8(0, 0, 0)),
                    crate::properties::ThemeProperty::ColorDisabled => Some(Color::from_rgb8(150, 150, 150)),
                    crate::properties::ThemeProperty::ColorMenuSelected => Some(Color::from_rgb8(100, 150, 255)),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(Color::from_rgb8(220, 220, 220)),
                    _ => None,
                },
                "MenuPopup" => match property {
                    crate::properties::ThemeProperty::ColorBackground => Some(Color::from_rgb8(245, 245, 245)),
                    crate::properties::ThemeProperty::ColorBorder => Some(Color::from_rgb8(200, 200, 200)),
                    crate::properties::ThemeProperty::ColorText => Some(Color::from_rgb8(0, 0, 0)),
                    crate::properties::ThemeProperty::ColorDisabled => Some(Color::from_rgb8(150, 150, 150)),
                    crate::properties::ThemeProperty::ColorMenuHovered => Some(Color::from_rgb8(220, 220, 220)),
                    crate::properties::ThemeProperty::ColorMenuDisabled => Some(Color::from_rgb8(150, 150, 150)),
                    _ => None,
                },
                "Toggle" => match property {
                    crate::properties::ThemeProperty::ColorToggleTrackOn => Some(Color::from_rgb8(100, 150, 255)),
                    crate::properties::ThemeProperty::ColorToggleTrackOff => Some(Color::from_rgb8(240, 240, 240)),
                    crate::properties::ThemeProperty::ColorToggleTrackBorder => Some(Color::from_rgb8(180, 180, 180)),
                    crate::properties::ThemeProperty::ColorToggleThumb => Some(Color::from_rgb8(255, 255, 255)),
                    crate::properties::ThemeProperty::ColorToggleThumbBorder => Some(Color::from_rgb8(180, 180, 180)),
                    crate::properties::ThemeProperty::ColorToggleDisabled => Some(Color::from_rgb8(200, 200, 200)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn window_background(&self) -> Color {
        Color::WHITE
    }

    fn globals(&self) -> &Globals {
        match &self {
            CelesteTheme::Light(globals) => globals,
        }
    }

    fn globals_mut(&mut self) -> &mut Globals {
        match self {
            CelesteTheme::Light(globals) => globals,
        }
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
