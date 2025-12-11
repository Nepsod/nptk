use crate::widget::BoxedWidget;
use std::sync::{Arc, Mutex};

/// A request to create a new popup window.
pub struct PopupRequest {
    pub widget: BoxedWidget,
    pub title: String,
    pub size: (u32, u32),
    pub position: Option<(i32, i32)>,
}

/// Manages requests for creating popup windows.
#[derive(Clone, Default)]
pub struct PopupManager {
    requests: Arc<Mutex<Vec<PopupRequest>>>,
}

impl PopupManager {
    /// Create a new PopupManager.
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request a new popup window.
    pub fn create_popup(&self, widget: BoxedWidget, title: impl Into<String>, size: (u32, u32)) {
        let mut requests = self.requests.lock().unwrap();
        requests.push(PopupRequest {
            widget,
            title: title.into(),
            size,
            position: None,
        });
    }

    /// Request a new popup window with a specific position.
    pub fn create_popup_at(
        &self,
        widget: BoxedWidget,
        title: impl Into<String>,
        size: (u32, u32),
        position: (i32, i32),
    ) {
        let mut requests = self.requests.lock().unwrap();
        requests.push(PopupRequest {
            widget,
            title: title.into(),
            size,
            position: Some(position),
        });
    }

    /// Drain all pending requests.
    pub(crate) fn drain_requests(&self) -> Vec<PopupRequest> {
        let mut requests = self.requests.lock().unwrap();
        std::mem::take(&mut *requests)
    }
}
