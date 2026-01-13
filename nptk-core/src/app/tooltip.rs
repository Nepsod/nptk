use nptk_theme::id::WidgetId;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Tooltip show delay in milliseconds.
const TOOLTIP_SHOW_DELAY_MS: u64 = 700;

/// Tooltip hide delay in milliseconds.
const TOOLTIP_HIDE_DELAY_MS: u64 = 50;

/// Minimum time between tooltip state changes to prevent rapid cycling.
const TOOLTIP_DEBOUNCE_MS: u64 = 100;

/// A tooltip request from a widget.
#[derive(Debug, Clone)]
pub enum TooltipRequest {
    /// Request to show a tooltip.
    Show {
        text: String,
        source_widget_id: WidgetId,
        cursor_pos: (f64, f64),
    },
    /// Request to hide the tooltip.
    Hide,
}

/// State of the tooltip system.
#[derive(Debug, Clone, PartialEq)]
enum TooltipState {
    /// No tooltip active.
    Idle,
    /// Tooltip requested, waiting for show delay to expire.
    PendingShow {
        text: String,
        source_widget_id: WidgetId,
        cursor_pos: (f64, f64),
        show_at: Instant,
    },
    /// Tooltip is currently showing.
    Showing {
        text: String,
        source_widget_id: WidgetId,
        cursor_pos: (f64, f64),
    },
    /// Tooltip hide requested, waiting for hide delay to expire.
    PendingHide {
        hide_at: Instant,
    },
}

/// Manages tooltip requests from widgets (similar to PopupManager).
#[derive(Clone, Default)]
pub struct TooltipRequestManager {
    requests: Arc<Mutex<Vec<TooltipRequest>>>,
}

impl TooltipRequestManager {
    /// Create a new TooltipRequestManager.
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request to show a tooltip.
    pub fn request_show(
        &self,
        text: String,
        source_widget_id: WidgetId,
        cursor_pos: (f64, f64),
    ) {
        let mut requests = self.requests.lock().unwrap();
        requests.push(TooltipRequest::Show {
            text,
            source_widget_id,
            cursor_pos,
        });
    }

    /// Request to hide the tooltip.
    pub fn request_hide(&self) {
        let mut requests = self.requests.lock().unwrap();
        requests.push(TooltipRequest::Hide);
    }

    /// Drain all pending requests.
    pub(crate) fn drain_requests(&self) -> Vec<TooltipRequest> {
        let mut requests = self.requests.lock().unwrap();
        std::mem::take(&mut *requests)
    }
}

/// Manages tooltip state and timing (owned by AppHandler).
pub struct TooltipManager {
    state: TooltipState,
    last_state_change: Instant,
}

impl TooltipManager {
    /// Create a new TooltipManager.
    pub fn new() -> Self {
        Self {
            state: TooltipState::Idle,
            last_state_change: Instant::now(),
        }
    }

    /// Process tooltip requests.
    ///
    /// This should be called with requests from TooltipRequestManager.
    pub fn process_requests(&mut self, requests: Vec<TooltipRequest>) {
        for request in requests {
            match request {
                TooltipRequest::Show {
                    text,
                    source_widget_id,
                    cursor_pos,
                } => {
                    self.request_show(text, source_widget_id, cursor_pos);
                },
                TooltipRequest::Hide => {
                    self.request_hide();
                },
            }
        }
    }

    /// Request to show a tooltip.
    ///
    /// If a tooltip is already showing, it will be updated immediately.
    /// If a tooltip is already pending for the same tooltip, only the cursor position is updated.
    /// Otherwise, the tooltip will be shown after the show delay.
    fn request_show(
        &mut self,
        text: String,
        source_widget_id: WidgetId,
        cursor_pos: (f64, f64),
    ) {
        let now = Instant::now();
        match &self.state {
            TooltipState::Showing { text: existing_text, source_widget_id: existing_id, .. } => {
                // Already showing - if different text/widget, hide current and start new timer
                if *existing_text != text || *existing_id != source_widget_id {
                    let show_at = now + Duration::from_millis(TOOLTIP_SHOW_DELAY_MS);
                    self.state = TooltipState::PendingShow {
                        text,
                        source_widget_id,
                        cursor_pos,
                        show_at,
                    };
                    self.last_state_change = now;
                } else {
                    // Same tooltip, just update cursor position
                    self.state = TooltipState::Showing {
                        text: existing_text.clone(),
                        source_widget_id: existing_id.clone(),
                        cursor_pos,
                    };
                }
            },
            TooltipState::PendingShow { text: existing_text, source_widget_id: existing_id, show_at, .. } => {
                // Already pending - only update cursor position, don't reset timer
                if *existing_text == text && *existing_id == source_widget_id {
                    // Same tooltip, just update cursor position
                    self.state = TooltipState::PendingShow {
                        text: existing_text.clone(),
                        source_widget_id: existing_id.clone(),
                        cursor_pos,
                        show_at: *show_at,
                    };
                } else {
                    let show_at = now + Duration::from_millis(TOOLTIP_SHOW_DELAY_MS);
                    self.state = TooltipState::PendingShow {
                        text,
                        source_widget_id,
                        cursor_pos,
                        show_at,
                    };
                    self.last_state_change = now;
                }
            },
            TooltipState::Idle | TooltipState::PendingHide { .. } => {
                let show_at = now + Duration::from_millis(TOOLTIP_SHOW_DELAY_MS);
                self.state = TooltipState::PendingShow {
                    text,
                    source_widget_id,
                    cursor_pos,
                    show_at,
                };
                self.last_state_change = now;
            },
        }
    }

    /// Request to hide the tooltip.
    ///
    /// If a tooltip is showing, it will be hidden after the hide delay.
    /// If a tooltip is pending show, it will be cancelled immediately.
    fn request_hide(&mut self) {
        let now = Instant::now();
        
        // Debounce rapid hide requests
        if now.duration_since(self.last_state_change) < Duration::from_millis(TOOLTIP_DEBOUNCE_MS) {
            return;
        }
        
        match &self.state {
            TooltipState::Idle | TooltipState::PendingHide { .. } => {
                // Already idle or hiding
            },
            TooltipState::PendingShow { .. } => {
                self.state = TooltipState::Idle;
                self.last_state_change = now;
            },
            TooltipState::Showing { .. } => {
                let hide_at = now + Duration::from_millis(TOOLTIP_HIDE_DELAY_MS);
                self.state = TooltipState::PendingHide { hide_at };
                self.last_state_change = now;
            },
        }
    }

    /// Update the tooltip state based on timers.
    ///
    /// Returns true if the tooltip state changed (needs redraw).
    pub fn update(&mut self, now: Instant) -> bool {
        match &self.state {
            TooltipState::Idle | TooltipState::Showing { .. } => {
                // No timer to check
                false
            },
            TooltipState::PendingShow { show_at, text, source_widget_id, cursor_pos } => {
                if now >= *show_at {
                    self.state = TooltipState::Showing {
                        text: text.clone(),
                        source_widget_id: source_widget_id.clone(),
                        cursor_pos: *cursor_pos,
                    };
                    self.last_state_change = now;
                    true
                } else {
                    false
                }
            },
            TooltipState::PendingHide { hide_at } => {
                if now >= *hide_at {
                    self.state = TooltipState::Idle;
                    self.last_state_change = now;
                    true
                } else {
                    false
                }
            },
        }
    }

    /// Get the current tooltip text if a tooltip is showing.
    pub fn current_tooltip(&self) -> Option<(&String, &WidgetId, (f64, f64))> {
        match &self.state {
            TooltipState::Showing { text, source_widget_id, cursor_pos } => {
                Some((text, source_widget_id, *cursor_pos))
            },
            _ => None,
        }
    }

    /// Check if a tooltip is currently showing.
    pub fn is_showing(&self) -> bool {
        matches!(&self.state, TooltipState::Showing { .. })
    }
}

impl Default for TooltipManager {
    fn default() -> Self {
        Self::new()
    }
}
