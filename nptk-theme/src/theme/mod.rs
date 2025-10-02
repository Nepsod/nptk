use peniko::Color;

use crate::globals::Globals;
use crate::id::WidgetId;
use crate::properties::{ThemeProperty, ThemeStyle, ThemeVariables};
use crate::style::{DefaultStyles, Style};

/// The Celeste Theme.
pub mod celeste;
/// The Dark Theme.
pub mod dark;

/// Base trait for all themes.
pub trait Theme {
    /// Return the [Style] of the given widget using its ID.
    /// Returns [None] if the theme does not have styles for the given widget.
    /// In that case, you should use [Theme::defaults] to get widget style defaults.
    fn of(&self, id: WidgetId) -> Option<Style>;
    
    /// Return the type-safe [ThemeStyle] of the given widget using its ID.
    /// Returns [None] if the theme does not have styles for the given widget.
    /// This is the preferred method for accessing theme properties.
    fn style(&self, id: WidgetId) -> Option<ThemeStyle> {
        // Default implementation converts from legacy Style
        self.of(id).map(|_style| {
            let theme_style = ThemeStyle::new();
            // Convert legacy style properties to type-safe properties
            // This is a fallback for backward compatibility
            theme_style
        })
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
            ThemeProperty::Color | ThemeProperty::Text => Some(self.defaults().text().foreground()),
            ThemeProperty::Background => Some(self.defaults().container().background()),
            ThemeProperty::Border => Some(Color::from_rgb8(200, 200, 200)),
            ThemeProperty::ColorIdle => Some(self.defaults().interactive().inactive()),
            ThemeProperty::ColorHovered => Some(self.defaults().interactive().hover()),
            ThemeProperty::ColorPressed => Some(self.defaults().interactive().active()),
            ThemeProperty::ColorDisabled => Some(self.defaults().interactive().disabled()),
            _ => None,
        }
    }
    
    /// Get the default widget styles.
    fn defaults(&self) -> DefaultStyles;
    
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
        self.of(id).is_some()
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
}
