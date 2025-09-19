use crate::app::focus::{FocusId, FocusProperties, FocusState};
use crate::layout::LayoutNode;

/// Trait for widgets that can receive focus.
pub trait Focusable {
    /// Get the focus ID for this widget.
    fn focus_id(&self) -> FocusId;
    
    /// Get the focus properties for this widget.
    fn focus_properties(&self) -> FocusProperties;
    
    /// Get the current focus state.
    fn focus_state(&self) -> FocusState;
    
    /// Called when this widget gains focus.
    fn on_focus_gained(&mut self) {}
    
    /// Called when this widget loses focus.
    fn on_focus_lost(&mut self) {}
    
    /// Called when this widget receives keyboard input (only when focused).
    fn on_key_input(&mut self, _event: &winit::event::KeyEvent) -> bool {
        // Default implementation does nothing
        false
    }
    
    /// Update focus bounds from layout information.
    fn update_focus_bounds(&mut self, layout: &LayoutNode);
}

/// Extension trait to add focus capabilities to any widget.
pub trait WidgetFocusExt {
    /// Make this widget focusable with the given properties.
    fn with_focus(self, properties: FocusProperties) -> FocusableWrapper<Self>
    where
        Self: Sized;
    
    /// Make this widget focusable with default properties.
    fn focusable(self) -> FocusableWrapper<Self>
    where
        Self: Sized,
    {
        self.with_focus(FocusProperties::default())
    }
}

impl<W> WidgetFocusExt for W {
    fn with_focus(self, properties: FocusProperties) -> FocusableWrapper<Self> {
        FocusableWrapper::new(self, properties)
    }
}

/// Wrapper that adds focus capabilities to any widget.
pub struct FocusableWrapper<W> {
    widget: W,
    focus_id: FocusId,
    properties: FocusProperties,
    focus_state: FocusState,
}

impl<W> FocusableWrapper<W> {
    /// Create a new focusable wrapper.
    pub fn new(widget: W, properties: FocusProperties) -> Self {
        Self {
            widget,
            focus_id: FocusId::new(),
            properties,
            focus_state: FocusState::None,
        }
    }
    
    /// Get access to the inner widget.
    pub fn inner(&self) -> &W {
        &self.widget
    }
    
    /// Get mutable access to the inner widget.
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.widget
    }
}

impl<W> Focusable for FocusableWrapper<W> {
    fn focus_id(&self) -> FocusId {
        self.focus_id
    }
    
    fn focus_properties(&self) -> FocusProperties {
        self.properties.clone()
    }
    
    fn focus_state(&self) -> FocusState {
        self.focus_state
    }
    
    fn update_focus_bounds(&mut self, layout: &LayoutNode) {
        use crate::app::focus::FocusBounds;
        
        let _bounds = FocusBounds {
            x: layout.layout.location.x,
            y: layout.layout.location.y,
            width: layout.layout.size.width,
            height: layout.layout.size.height,
        };
        
        // This would need to be integrated with the focus manager
        // For now, we'll store the bounds in the wrapper
    }
}

// We'll need to implement the Widget trait for FocusableWrapper
// This will be done when we implement the Widget integration
