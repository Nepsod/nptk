//! Lightweight data structures shared by rendering helpers.
use super::state::WidgetState;
use crate::id::WidgetId;

/// Dimensions for a widget render pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderBounds {
    /// Width in logical pixels.
    pub width: f32,
    /// Height in logical pixels.
    pub height: f32,
}

impl RenderBounds {
    /// Create bounds with explicit width and height.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Convenience helper for square bounds.
    pub fn square(size: f32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }
}

impl Default for RenderBounds {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

/// Lightweight context passed to rendering helpers.
#[derive(Debug)]
pub struct RenderContext<'a, T> {
    widget_id: WidgetId,
    state: WidgetState,
    bounds: RenderBounds,
    payload: Option<&'a T>,
}

impl<'a, T> RenderContext<'a, T> {
    /// Build a context scoped to a widget id and state.
    pub fn new(widget_id: WidgetId, state: WidgetState) -> Self {
        Self {
            widget_id,
            state,
            bounds: RenderBounds::default(),
            payload: None,
        }
    }

    /// Attach bounds metadata to the context.
    pub fn with_bounds(mut self, bounds: RenderBounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Attach a payload such as a graphics backend specific handle.
    pub fn with_payload(mut self, payload: &'a T) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Access the widget identifier associated with this render pass.
    pub fn widget_id(&self) -> &WidgetId {
        &self.widget_id
    }

    /// Retrieve the widget's interaction state.
    pub fn state(&self) -> WidgetState {
        self.state
    }

    /// Get the bounds metadata assigned to the context.
    pub fn bounds(&self) -> RenderBounds {
        self.bounds
    }

    /// Optional payload carried alongside render metadata.
    pub fn payload(&self) -> Option<&'a T> {
        self.payload
    }
}
