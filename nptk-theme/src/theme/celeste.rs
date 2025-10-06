use peniko::{color::palette, Color};

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::style::{
    DefaultContainerStyles, DefaultInteractiveStyles, DefaultStyles, DefaultTextStyles, Style,
    StyleVal,
};
use crate::theme::Theme;
use crate::rendering::ThemeRenderer;

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
    fn of(&self, id: WidgetId) -> Option<Style> {
        match id.namespace() {
            "nptk-widgets" => match id.id() {
                "Text" => Some(Style::from_values([
                    ("color".to_string(), StyleVal::Color(palette::css::BLACK)),
                    (
                        "color_invert".to_string(),
                        StyleVal::Color(palette::css::WHITE),
                    ),
                ])),

                "Button" => Some(Style::from_values([
                    (
                        "color_idle".to_string(),
                        StyleVal::Color(Color::from_rgb8(150, 170, 250)),
                    ),
                    (
                        "color_pressed".to_string(),
                        StyleVal::Color(Color::from_rgb8(130, 150, 230)),
                    ),
                    (
                        "color_hovered".to_string(),
                        StyleVal::Color(Color::from_rgb8(140, 160, 240)),
                    ),
                    (
                        "color_focused".to_string(),
                        StyleVal::Color(Color::from_rgb8(120, 140, 220)),
                    ),
                ])),

                "Checkbox" => Some(Style::from_values([
                    (
                        "color_checked".to_string(),
                        StyleVal::Color(Color::from_rgb8(130, 130, 230)),
                    ),
                    (
                        "color_unchecked".to_string(),
                        StyleVal::Color(Color::from_rgb8(170, 170, 250)),
                    ),
                ])),

                "Slider" => Some(Style::from_values([
                    (
                        "color".to_string(),
                        StyleVal::Color(Color::from_rgb8(130, 130, 230)),
                    ),
                    (
                        "color_ball".to_string(),
                        StyleVal::Color(Color::from_rgb8(170, 170, 250)),
                    ),
                ])),

                "TextInput" => Some(Style::from_values([
                    (
                        "color_background".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "color_border".to_string(),
                        StyleVal::Color(Color::from_rgb8(200, 200, 200)),
                    ),
                    (
                        "color_border_focused".to_string(),
                        StyleVal::Color(Color::from_rgb8(100, 150, 255)),
                    ),
                    (
                        "color_text".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_cursor".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_selection".to_string(),
                        StyleVal::Color(Color::from_rgb8(180, 200, 255)),
                    ),
                ])),

                "SecretInput" => Some(Style::from_values([
                    (
                        "color_background".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "color_border".to_string(),
                        StyleVal::Color(Color::from_rgb8(200, 200, 200)),
                    ),
                    (
                        "color_border_focused".to_string(),
                        StyleVal::Color(Color::from_rgb8(100, 150, 255)),
                    ),
                    (
                        "color_text".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_cursor".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_selection".to_string(),
                        StyleVal::Color(Color::from_rgb8(180, 200, 255)),
                    ),
                ])),

                "ValueInput" => Some(Style::from_values([
                    (
                        "color_background".to_string(),
                        StyleVal::Color(Color::from_rgb8(250, 250, 250)),
                    ),
                    (
                        "color_background_focused".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "color_border".to_string(),
                        StyleVal::Color(Color::from_rgb8(200, 200, 200)),
                    ),
                    (
                        "color_border_focused".to_string(),
                        StyleVal::Color(Color::from_rgb8(0, 120, 255)),
                    ),
                    (
                        "color_border_error".to_string(),
                        StyleVal::Color(Color::from_rgb8(255, 0, 0)),
                    ),
                    (
                        "color_text".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_cursor".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_placeholder".to_string(),
                        StyleVal::Color(Color::from_rgb8(150, 150, 150)),
                    ),
                ])),

                "RadioButton" => Some(Style::from_values([
                    (
                        "color_background".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "color_background_selected".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "color_background_disabled".to_string(),
                        StyleVal::Color(Color::from_rgb8(240, 240, 240)),
                    ),
                    (
                        "color_border".to_string(),
                        StyleVal::Color(Color::from_rgb8(150, 150, 150)),
                    ),
                    (
                        "color_border_hovered".to_string(),
                        StyleVal::Color(Color::from_rgb8(100, 100, 100)),
                    ),
                    (
                        "color_border_focused".to_string(),
                        StyleVal::Color(Color::from_rgb8(0, 120, 255)),
                    ),
                    (
                        "color_border_disabled".to_string(),
                        StyleVal::Color(Color::from_rgb8(200, 200, 200)),
                    ),
                    (
                        "color_dot".to_string(),
                        StyleVal::Color(Color::from_rgb8(0, 120, 255)),
                    ),
                    (
                        "color_dot_disabled".to_string(),
                        StyleVal::Color(Color::from_rgb8(180, 180, 180)),
                    ),
                    (
                        "color_text".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_text_disabled".to_string(),
                        StyleVal::Color(Color::from_rgb8(150, 150, 150)),
                    ),
                ])),

                "MenuBar" => Some(Style::from_values([
                    (
                        "color_background".to_string(),
                        StyleVal::Color(Color::from_rgb8(240, 240, 240)),
                    ),
                    (
                        "color_text".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                    (
                        "color_hovered".to_string(),
                        StyleVal::Color(Color::from_rgb8(220, 220, 220)),
                    ),
                    (
                        "color_selected".to_string(),
                        StyleVal::Color(Color::from_rgb8(70, 130, 255)),
                    ),
                    (
                        "color_disabled".to_string(),
                        StyleVal::Color(Color::from_rgb8(150, 150, 150)),
                    ),
                    (
                        "color_border".to_string(),
                        StyleVal::Color(Color::from_rgb8(200, 200, 200)),
                    ),
                ])),

                "ScrollContainer" => Some(Style::from_values([
                    (
                        "color_background".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "color_border".to_string(),
                        StyleVal::Color(Color::from_rgb8(200, 200, 200)),
                    ),
                    (
                        "color_scrollbar".to_string(),
                        StyleVal::Color(Color::from_rgb8(230, 230, 230)),
                    ),
                    (
                        "color_scrollbar_thumb".to_string(),
                        StyleVal::Color(Color::from_rgb8(180, 180, 180)),
                    ),
                    (
                        "color_scrollbar_thumb_hover".to_string(),
                        StyleVal::Color(Color::from_rgb8(150, 150, 150)),
                    ),
                    (
                        "color_scrollbar_thumb_active".to_string(),
                        StyleVal::Color(Color::from_rgb8(120, 120, 120)),
                    ),
                ])),

                "TabsContainer" => Some(Style::from_values([
                    (
                        "background".to_string(),
                        StyleVal::Color(Color::from_rgb8(245, 245, 245)),
                    ),
                    (
                        "border".to_string(),
                        StyleVal::Color(Color::from_rgb8(180, 180, 180)),
                    ),
                    (
                        "tab_bar_background".to_string(),
                        StyleVal::Color(Color::from_rgb8(250, 250, 250)),
                    ),
                    (
                        "content_background".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "tab_active".to_string(),
                        StyleVal::Color(Color::WHITE),
                    ),
                    (
                        "tab_inactive".to_string(),
                        StyleVal::Color(Color::from_rgb8(230, 230, 230)),
                    ),
                    (
                        "tab_hovered".to_string(),
                        StyleVal::Color(Color::from_rgb8(240, 240, 240)),
                    ),
                    (
                        "tab_pressed".to_string(),
                        StyleVal::Color(Color::from_rgb8(210, 210, 210)),
                    ),
                    (
                        "tab_text".to_string(),
                        StyleVal::Color(Color::from_rgb8(50, 50, 50)),
                    ),
                    (
                        "tab_text_active".to_string(),
                        StyleVal::Color(Color::BLACK),
                    ),
                ])),

                _ => None,
            },
            _ => None,
        }
    }

    fn defaults(&self) -> DefaultStyles {
        DefaultStyles::new(
            DefaultTextStyles::new(palette::css::BLACK, palette::css::WHITE_SMOKE),
            DefaultContainerStyles::new(palette::css::ANTIQUE_WHITE, palette::css::WHITE),
            DefaultInteractiveStyles::new(
                Color::from_rgb8(130, 150, 230),
                Color::from_rgb8(150, 170, 250),
                Color::from_rgb8(140, 160, 240),
                Color::from_rgb8(110, 110, 110),
            ),
        )
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
    
    fn supports_rendering(&self) -> bool {
        true
    }
    
    fn as_renderer(&mut self) -> Option<&mut dyn ThemeRenderer> {
        Some(self)
    }
}

impl ThemeRenderer for CelesteTheme {
    // Use default implementations from the trait
}
