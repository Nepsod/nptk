use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Unique identifier for focusable widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(pub u64);

impl FocusId {
    /// Generate a new unique focus ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Focus state for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusState {
    /// Widget is not focused.
    None,
    /// Widget has focus.
    Focused,
    /// Widget had focus but lost it this frame.
    Lost,
    /// Widget gained focus this frame.
    Gained,
}

/// Properties that describe how a widget can receive focus.
#[derive(Debug, Clone)]
pub struct FocusProperties {
    /// Whether this widget can receive focus via tab navigation.
    pub tab_focusable: bool,
    /// Whether this widget can receive focus via mouse click.
    pub click_focusable: bool,
    /// Tab order index (lower numbers get focus first).
    pub tab_index: i32,
    /// Whether this widget accepts keyboard input.
    pub accepts_keyboard: bool,
}

impl Default for FocusProperties {
    fn default() -> Self {
        Self {
            tab_focusable: true,
            click_focusable: true,
            tab_index: 0,
            accepts_keyboard: false,
        }
    }
}

/// Widget bounds for focus hit testing.
#[derive(Debug, Clone)]
pub struct FocusBounds {
    /// X coordinate of the bounds.
    pub x: f32,
    /// Y coordinate of the bounds.
    pub y: f32,
    /// Width of the bounds.
    pub width: f32,
    /// Height of the bounds.
    pub height: f32,
}

impl FocusBounds {
    /// Check if a point is within these bounds.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && x <= (self.x + self.width) as f64
            && y >= self.y as f64
            && y <= (self.y + self.height) as f64
    }
}

/// Information about a focusable widget.
#[derive(Debug, Clone)]
pub struct FocusableWidget {
    /// Unique identifier for the widget.
    pub id: FocusId,
    /// Focus properties for the widget.
    pub properties: FocusProperties,
    /// Current bounds of the widget.
    pub bounds: FocusBounds,
}

/// Global focus manager that tracks focus state across all widgets.
#[derive(Debug)]
pub struct FocusManager {
    /// Currently focused widget ID.
    focused_widget: Option<FocusId>,
    /// Widget that had focus in the previous frame.
    previous_focused: Option<FocusId>,
    /// Registry of all focusable widgets.
    widgets: HashMap<FocusId, FocusableWidget>,
    /// Ordered list of widgets for tab navigation.
    tab_order: Vec<FocusId>,
    /// Whether tab order needs to be recalculated.
    tab_order_dirty: bool,
    /// Whether the last focus change was via keyboard.
    last_focus_via_keyboard: bool,
}

impl FocusManager {
    /// Create a new focus manager.
    pub fn new() -> Self {
        Self {
            focused_widget: None,
            previous_focused: None,
            widgets: HashMap::new(),
            tab_order: Vec::new(),
            tab_order_dirty: false,
            last_focus_via_keyboard: false,
        }
    }

    /// Register a focusable widget.
    pub fn register_widget(&mut self, widget: FocusableWidget) {
        self.widgets.insert(widget.id, widget);
        self.tab_order_dirty = true;
    }

    /// Unregister a focusable widget.
    pub fn unregister_widget(&mut self, id: FocusId) {
        self.widgets.remove(&id);
        if self.focused_widget == Some(id) {
            self.focused_widget = None;
        }
        self.tab_order_dirty = true;
    }

    /// Update widget bounds (called when layout changes).
    pub fn update_widget_bounds(&mut self, id: FocusId, bounds: FocusBounds) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.bounds = bounds;
        }
    }

    /// Get the current focus state for a widget.
    pub fn get_focus_state(&self, id: FocusId) -> FocusState {
        let is_focused = self.focused_widget == Some(id);
        let was_focused = self.previous_focused == Some(id);

        match (was_focused, is_focused) {
            (false, true) => FocusState::Gained,
            (true, false) => FocusState::Lost,
            (true, true) => FocusState::Focused,
            (false, false) => FocusState::None,
        }
    }

    /// Set focus to a specific widget.
    pub fn set_focus(&mut self, id: Option<FocusId>) {
        self.set_focus_internal(id, false);
    }

    /// Set focus to a specific widget via keyboard.
    pub fn set_focus_via_keyboard(&mut self, id: Option<FocusId>) {
        self.set_focus_internal(id, true);
    }

    /// Internal method to set focus with keyboard flag.
    fn set_focus_internal(&mut self, id: Option<FocusId>, via_keyboard: bool) {
        self.previous_focused = self.focused_widget;
        self.focused_widget = id;
        self.last_focus_via_keyboard = via_keyboard;
    }

    /// Get the currently focused widget ID.
    pub fn get_focused_widget(&self) -> Option<FocusId> {
        self.focused_widget
    }

    /// Check if the last focus change was via keyboard.
    pub fn was_last_focus_via_keyboard(&self) -> bool {
        self.last_focus_via_keyboard
    }

    /// Move focus to the next widget in tab order.
    pub fn focus_next(&mut self) {
        if let Some(next_id) = self.find_next_in_tab_order() {
            self.set_focus_via_keyboard(Some(next_id));
        }
    }

    /// Move focus to the previous widget in tab order.
    pub fn focus_previous(&mut self) {
        if let Some(prev_id) = self.find_previous_in_tab_order() {
            self.set_focus_via_keyboard(Some(prev_id));
        }
    }

    /// Find the next widget ID in tab order.
    fn find_next_in_tab_order(&mut self) -> Option<FocusId> {
        self.update_tab_order();

        if self.tab_order.is_empty() {
            return None;
        }

        let current_index = self.find_current_tab_index();
        let next_index = (current_index + 1) % self.tab_order.len();
        Some(self.tab_order[next_index])
    }

    /// Find the previous widget ID in tab order.
    fn find_previous_in_tab_order(&mut self) -> Option<FocusId> {
        self.update_tab_order();

        if self.tab_order.is_empty() {
            return None;
        }

        let current_index = self.find_current_tab_index();
        let prev_index = if current_index == 0 {
            self.tab_order.len() - 1
        } else {
            current_index - 1
        };
        Some(self.tab_order[prev_index])
    }

    /// Find the current widget's index in the tab order.
    fn find_current_tab_index(&self) -> usize {
        self.focused_widget
            .and_then(|id| self.tab_order.iter().position(|&x| x == id))
            .unwrap_or(0)
    }

    /// Handle mouse click for focus.
    pub fn handle_click(&mut self, x: f64, y: f64) -> bool {
        if let Some(widget_id) = self.find_widget_at_position(x, y) {
            self.set_focus(Some(widget_id));
            true
        } else {
            self.set_focus(None);
            false
        }
    }

    /// Find the widget at the given position that can receive click focus.
    fn find_widget_at_position(&self, x: f64, y: f64) -> Option<FocusId> {
        self.widgets
            .values()
            .find(|widget| widget.properties.click_focusable && widget.bounds.contains(x, y))
            .map(|widget| widget.id)
    }

    /// Update the tab order based on current widgets.
    fn update_tab_order(&mut self) {
        if !self.tab_order_dirty {
            return;
        }

        self.tab_order.clear();

        let mut tab_widgets: Vec<_> = self
            .widgets
            .values()
            .filter(|w| w.properties.tab_focusable)
            .collect();

        tab_widgets.sort_by_key(|w| (w.properties.tab_index, w.id.0));

        self.tab_order.extend(tab_widgets.iter().map(|w| w.id));
        self.tab_order_dirty = false;
    }

    /// Clear focus from all widgets.
    pub fn clear_focus(&mut self) {
        self.set_focus(None);
    }

    /// Prepare for next frame (update previous focus state).
    pub fn next_frame(&mut self) {
        // This is called at the beginning of each frame to update focus transitions
        self.previous_focused = self.focused_widget;
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper around FocusManager.
pub type SharedFocusManager = Arc<Mutex<FocusManager>>;
