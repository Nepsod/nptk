/// Basic layout metrics shared across themes.
#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    /// Default corner radius for controls.
    pub control_corner_radius: f32,
    /// Focus ring stroke width.
    pub focus_ring_width: f32,
    /// Whether text prefers inverted colors on controls.
    pub prefers_inverted_text: bool,
}

/// Trait used by themes that expose layout metrics.
pub trait ProvidesLayoutMetrics {
    /// Fetch the metrics snapshot.
    fn layout_metrics(&self) -> LayoutMetrics;
}

impl LayoutMetrics {
    /// Metrics for a classic light appearance.
    pub fn classic_light() -> Self {
        Self {
            control_corner_radius: 3.0,
            focus_ring_width: 1.0,
            prefers_inverted_text: false,
        }
    }

    /// Metrics tuned for the dark theme.
    pub fn modern_dark() -> Self {
        Self {
            control_corner_radius: 6.0,
            focus_ring_width: 1.5,
            prefers_inverted_text: true,
        }
    }

    /// Metrics used by the vibrant sweet theme.
    pub fn vibrant_dark() -> Self {
        Self {
            control_corner_radius: 5.0,
            focus_ring_width: 1.2,
            prefers_inverted_text: true,
        }
    }
}

