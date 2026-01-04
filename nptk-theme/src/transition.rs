//! # Theme Transitions
//!
//! This module provides smooth transition support for theme switching,
//! allowing colors to interpolate between themes over a configurable duration.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::id::WidgetId;
use crate::properties::ThemeProperty;
use crate::theme::Theme;
use vello::peniko::Color;

/// Configuration for theme transitions.
#[derive(Debug, Clone)]
pub struct TransitionConfig {
    /// Whether transitions are enabled.
    pub enabled: bool,
    /// Transition duration in milliseconds.
    pub duration_ms: u64,
}

impl TransitionConfig {
    /// Create a new transition configuration.
    pub fn new(enabled: bool, duration_ms: u64) -> Self {
        Self { enabled, duration_ms }
    }

    /// Get the transition duration.
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    /// Check if transitions are enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            duration_ms: 300,
        }
    }
}

/// State of an active theme transition.
pub struct ThemeTransition {
    /// The starting theme (before transition).
    start_theme: Box<dyn Theme + Send + Sync>,
    /// The target theme (after transition).
    target_theme: Box<dyn Theme + Send + Sync>,
    /// When the transition started.
    start_time: Instant,
    /// How long the transition should take.
    duration: Duration,
    /// Properties that are being transitioned (for optimization).
    properties_to_transition: HashSet<(WidgetId, ThemeProperty)>,
}

impl ThemeTransition {
    /// Create a new theme transition.
    pub fn new(
        start_theme: Box<dyn Theme + Send + Sync>,
        target_theme: Box<dyn Theme + Send + Sync>,
        duration: Duration,
    ) -> Self {
        Self {
            start_theme,
            target_theme,
            start_time: Instant::now(),
            duration,
            properties_to_transition: HashSet::new(),
        }
    }

    /// Check if the transition is complete.
    pub fn is_complete(&self) -> bool {
        self.elapsed() >= self.duration
    }

    /// Get the elapsed time since the transition started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get the progress of the transition (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        let elapsed = self.elapsed();
        if elapsed >= self.duration {
            1.0
        } else {
            elapsed.as_secs_f32() / self.duration.as_secs_f32()
        }
    }

    /// Get an interpolated color for a widget property during transition.
    ///
    /// Returns `None` if the property is not available in either theme,
    /// or if the transition is complete (use target theme instead).
    pub fn get_interpolated_color(
        &self,
        id: &WidgetId,
        property: &ThemeProperty,
    ) -> Option<Color> {
        if self.is_complete() {
            return None;
        }

        let start_color = self.start_theme.get_property(id.clone(), property)?;
        let target_color = self.target_theme.get_property(id.clone(), property)?;

        let progress = self.progress();
        Some(lerp_color(start_color, target_color, progress))
    }

    /// Get the starting theme.
    pub fn start_theme(&self) -> &dyn Theme {
        self.start_theme.as_ref()
    }

    /// Get the target theme.
    pub fn target_theme(&self) -> &dyn Theme {
        self.target_theme.as_ref()
    }

    /// Add a property to the transition set (for optimization).
    pub fn add_property(&mut self, id: WidgetId, property: ThemeProperty) {
        self.properties_to_transition.insert((id, property));
    }

    /// Check if a property is in the transition set.
    pub fn has_property(&self, id: &WidgetId, property: &ThemeProperty) -> bool {
        self.properties_to_transition.contains(&(id.clone(), *property))
    }
}

/// Linear interpolation between two colors.
///
/// `t` should be between 0.0 (start_color) and 1.0 (end_color).
fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let start_components = start.components;
    let end_components = end.components;

    let r = lerp(start_components[0], end_components[0], t);
    let g = lerp(start_components[1], end_components[1], t);
    let b = lerp(start_components[2], end_components[2], t);
    let a = lerp(start_components[3], end_components[3], t);

    Color::from_rgba8(
        (r * 255.0).clamp(0.0, 255.0) as u8,
        (g * 255.0).clamp(0.0, 255.0) as u8,
        (b * 255.0).clamp(0.0, 255.0) as u8,
        (a * 255.0).clamp(0.0, 255.0) as u8,
    )
}

/// Linear interpolation between two f32 values.
fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp_color() {
        let start = Color::from_rgb8(0, 0, 0);
        let end = Color::from_rgb8(255, 255, 255);

        let mid = lerp_color(start, end, 0.5);
        // Midpoint should be approximately gray
        assert!((mid.components[0] as f32) > 120.0 && (mid.components[0] as f32) < 140.0);
        assert!((mid.components[1] as f32) > 120.0 && (mid.components[1] as f32) < 140.0);
        assert!((mid.components[2] as f32) > 120.0 && (mid.components[2] as f32) < 140.0);
    }

    #[test]
    fn test_progress() {
        use crate::theme::{dark::DarkTheme, sweet::SweetTheme};
        let start_theme = Box::new(DarkTheme::new());
        let target_theme = Box::new(SweetTheme::new());
        let transition = ThemeTransition::new(start_theme, target_theme, Duration::from_millis(100));

        assert_eq!(transition.progress(), 0.0);
        assert!(!transition.is_complete());
    }
}
