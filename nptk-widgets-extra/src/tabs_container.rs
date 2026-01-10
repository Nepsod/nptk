// SPDX-License-Identifier: LGPL-3.0-only
use nptk_widgets::text::Text;
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{
    Affine, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke, Vec2,
};
use nptk_core::vg::peniko::{Brush, Color, Fill, Gradient, Mix};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::sync::{Arc, Mutex};

const TAB_CORNER_RADIUS: f64 = 3.0;
const ACCENT_INSET: f64 = 6.0;
const ACCENT_THICKNESS: f64 = 4.0;

/// Position of tabs in the TabsContainer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabPosition {
    /// Tabs at the top
    Top,
    /// Tabs at the bottom
    Bottom,
    /// Tabs on the left side
    Left,
    /// Tabs on the right side
    Right,
}

/// A single tab item
pub struct TabItem {
    /// Unique identifier for the tab
    pub id: String,
    /// Display label for the tab
    pub label: String,
    /// Content widget for this tab
    pub content: BoxedWidget,
    /// Whether the tab is enabled
    pub enabled: bool,
    /// Optional close button callback
    pub on_close: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
}

impl TabItem {
    /// Create a new tab item
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Widget + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            content: Box::new(content),
            enabled: true,
            on_close: None,
        }
    }

    /// Set whether the tab is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a close button with callback
    pub fn with_close_callback(
        mut self,
        callback: impl Fn() -> Update + Send + Sync + 'static,
    ) -> Self {
        self.on_close = Some(Arc::new(callback));
        self
    }
}

/// Error type for tab operations
#[derive(Debug, Clone)]
pub enum TabsError {
    /// Operation not allowed in static mode
    StaticMode,
    /// Tab ID not found
    TabNotFound(String),
}

/// Tab data for dynamic mode (simplified structure that can be cloned)
#[derive(Clone, Debug)]
pub struct TabData {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

/// Mode of operation for TabsContainer
enum TabsMode {
    /// Static mode: tabs are immutable after construction
    Static,
    /// Dynamic mode: tabs are managed via StateSignal
    /// Stores TabData (id, label) in signal, TabItem content is reconstructed
    Dynamic(
        StateSignal<Vec<TabData>>,
        Arc<Mutex<Vec<TabData>>>,
        Vec<TabItem>,
    ),
}

/// A container widget that displays tabs and switches between content
pub struct TabsContainer {
    /// Widget ID
    widget_id: WidgetId,
    /// Layout style
    layout_style: MaybeSignal<LayoutStyle>,
    /// Mode of operation (static or dynamic)
    mode: TabsMode,
    /// List of tabs (used in static mode, or as cache in dynamic mode)
    tabs: Vec<TabItem>,
    /// Currently active tab index
    active_tab: StateSignal<usize>,
    /// Position of tabs
    tab_position: TabPosition,
    /// Tab height (for horizontal tabs) or width (for vertical tabs)
    tab_size: f32,
    /// Mouse position
    mouse_pos: Vector2<f32>,
    /// Hovered tab index
    hovered_tab: Option<usize>,
    /// Pressed tab index
    pressed_tab: Option<usize>,
    /// Whether close buttons are hovered
    hovered_close: Option<usize>,
    /// Whether close buttons are pressed
    pressed_close: Option<usize>,

    // Scrolling
    scroll_offset: f32,
    max_scroll: f32,

    // Reordering
    dragging_tab: Option<usize>,
    drag_offset: f32, // Offset of mouse from tab start when drag began

    // Text rendering
    text_render_context: TextRenderContext,

    // Action button
    /// Callback for action button click
    action_button_callback: Option<Arc<dyn Fn() -> Update + Send + Sync>>,
    /// Whether action button is hovered
    action_button_hovered: bool,
    /// Size of action button
    action_button_size: f32,

    // Tab history (optional)
    /// Recently closed tabs (if history is enabled)
    tab_history: Option<Vec<TabItem>>,
    /// Whether history tracking is enabled
    history_enabled: bool,
    /// Maximum number of closed tabs to remember
    max_history_size: usize,
}

impl TabsContainer {
    /// Create a new TabsContainer in static mode
    pub fn new() -> Self {
        Self {
            widget_id: WidgetId::new("nptk-widgets", "TabsContainer"),
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            mode: TabsMode::Static,
            tabs: Vec::new(),
            active_tab: StateSignal::new(0),
            tab_position: TabPosition::Top,
            tab_size: 32.0,
            mouse_pos: Vector2::zeros(),
            hovered_tab: None,
            pressed_tab: None,
            hovered_close: None,
            pressed_close: None,
            scroll_offset: 0.0,
            max_scroll: 0.0,
            dragging_tab: None,
            drag_offset: 0.0,
            text_render_context: TextRenderContext::new(),
            action_button_callback: None,
            action_button_hovered: false,
            action_button_size: 32.0,
            tab_history: None,
            history_enabled: false,
            max_history_size: 10,
        }
    }

    /// Create a new TabsContainer in dynamic mode with reactive tab management
    /// Note: TabItem content widgets are stored separately and matched by ID
    pub fn new_dynamic(context: &AppContext, initial_tabs: Vec<TabItem>) -> Self {
        // Extract TabData from TabItems
        let tabs_data: Vec<TabData> = initial_tabs
            .iter()
            .map(|tab| TabData {
                id: tab.id.clone(),
                label: tab.label.clone(),
                enabled: tab.enabled,
            })
            .collect();

        let tabs_signal = context.use_state(tabs_data.clone());
        let tabs_shared = Arc::new(Mutex::new(tabs_data));

        Self {
            widget_id: WidgetId::new("nptk-widgets", "TabsContainer"),
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            mode: TabsMode::Dynamic(tabs_signal, tabs_shared, initial_tabs),
            tabs: Vec::new(), // Will be populated from signal
            active_tab: StateSignal::new(0),
            tab_position: TabPosition::Top,
            tab_size: 32.0,
            mouse_pos: Vector2::zeros(),
            hovered_tab: None,
            pressed_tab: None,
            hovered_close: None,
            pressed_close: None,
            scroll_offset: 0.0,
            max_scroll: 0.0,
            dragging_tab: None,
            drag_offset: 0.0,
            text_render_context: TextRenderContext::new(),
            action_button_callback: None,
            action_button_hovered: false,
            action_button_size: 32.0,
            tab_history: None,
            history_enabled: false,
            max_history_size: 10,
        }
    }

    /// Add a tab to the container (builder pattern, works in both modes)
    pub fn with_tab(mut self, tab: TabItem) -> Self {
        match &mut self.mode {
            TabsMode::Static => {
                self.tabs.push(tab);
            },
            TabsMode::Dynamic(signal, shared, content_store) => {
                let tab_data = TabData {
                    id: tab.id.clone(),
                    label: tab.label.clone(),
                    enabled: tab.enabled,
                };
                // Store content widget
                content_store.push(tab);
                // Update signal
                signal.mutate(|tabs| {
                    tabs.push(tab_data.clone());
                });
                if let Ok(mut shared_tabs) = shared.lock() {
                    *shared_tabs = signal.get().iter().cloned().collect();
                }
            },
        }
        self
    }

    /// Add a tab to the container (dynamic mode only)
    pub fn add_tab(&mut self, tab: TabItem) -> Result<(), TabsError> {
        match &mut self.mode {
            TabsMode::Static => Err(TabsError::StaticMode),
            TabsMode::Dynamic(signal, shared, content_store) => {
                let tab_data = TabData {
                    id: tab.id.clone(),
                    label: tab.label.clone(),
                    enabled: tab.enabled,
                };
                // Store content widget
                content_store.push(tab);
                // Update signal
                signal.mutate(|tabs| {
                    tabs.push(tab_data.clone());
                });
                if let Ok(mut shared_tabs) = shared.lock() {
                    *shared_tabs = signal.get().iter().cloned().collect();
                }
                self.validate_state();
                Ok(())
            },
        }
    }

    /// Remove a tab by ID (dynamic mode only)
    pub fn remove_tab(&mut self, id: &str) -> Result<(), TabsError> {
        match &mut self.mode {
            TabsMode::Static => Err(TabsError::StaticMode),
            TabsMode::Dynamic(signal, shared, content_store) => {
                let mut found = false;
                let mut removed_tab_data: Option<TabData> = None;

                signal.mutate(|tabs| {
                    if let Some(pos) = tabs.iter().position(|t| t.id == id) {
                        found = true;
                        removed_tab_data = Some(tabs.remove(pos).clone());
                    }
                });

                if !found {
                    return Err(TabsError::TabNotFound(id.to_string()));
                }

                // Remove from content store
                if let Some(tab_data) = &removed_tab_data {
                    if let Some(pos) = content_store.iter().position(|t| t.id == tab_data.id) {
                        let removed_tab = content_store.remove(pos);

                        // Add to history if enabled
                        if self.history_enabled {
                            if let Some(ref mut history) = self.tab_history {
                                history.push(removed_tab);
                                // Limit history size
                                if history.len() > self.max_history_size {
                                    history.remove(0);
                                }
                            } else {
                                self.tab_history = Some(vec![removed_tab]);
                            }
                        }
                    }
                }

                // Sync to shared - clone signal data first to avoid borrow issues
                let signal_data: Vec<TabData> = {
                    let signal_ref = signal.get();
                    signal_ref.iter().cloned().collect()
                };
                if let Ok(mut shared_tabs) = shared.lock() {
                    *shared_tabs = signal_data;
                }

                self.validate_state();
                Ok(())
            },
        }
    }

    /// Get the tabs signal (dynamic mode only)
    /// Returns signal containing TabData (id, label, enabled)
    pub fn get_tabs_signal(&self) -> Option<StateSignal<Vec<TabData>>> {
        match &self.mode {
            TabsMode::Static => None,
            TabsMode::Dynamic(signal, _, _) => Some(signal.clone()),
        }
    }

    /// Get the shared tabs state (dynamic mode only)
    /// Returns Arc<Mutex<Vec<TabData>>> for thread-safe access
    pub fn get_tabs_shared(&self) -> Option<Arc<Mutex<Vec<TabData>>>> {
        match &self.mode {
            TabsMode::Static => None,
            TabsMode::Dynamic(_, shared, _) => Some(shared.clone()),
        }
    }

    /// Add an action button callback (works in both modes)
    pub fn with_action_button(
        mut self,
        callback: impl Fn() -> Update + Send + Sync + 'static,
    ) -> Self {
        self.action_button_callback = Some(Arc::new(callback));
        self
    }

    /// Enable and configure tab history tracking
    pub fn with_history(mut self, enabled: bool, max_size: usize) -> Self {
        self.history_enabled = enabled;
        self.max_history_size = max_size;
        if enabled && self.tab_history.is_none() {
            self.tab_history = Some(Vec::new());
        }
        self
    }

    /// Get list of recently closed tabs
    pub fn get_closed_tabs(&self) -> Option<&[TabItem]> {
        self.tab_history.as_deref()
    }

    /// Reopen a tab from history (dynamic mode only)
    pub fn reopen_tab(&mut self, index: usize) -> Result<(), TabsError> {
        match &mut self.mode {
            TabsMode::Static => Err(TabsError::StaticMode),
            TabsMode::Dynamic(signal, shared, content_store) => {
                if let Some(ref mut history) = self.tab_history {
                    if index >= history.len() {
                        return Err(TabsError::TabNotFound(format!("History index {}", index)));
                    }
                    let tab = history.remove(index);
                    let tab_data = TabData {
                        id: tab.id.clone(),
                        label: tab.label.clone(),
                        enabled: tab.enabled,
                    };
                    // Store content widget
                    content_store.push(tab);
                    // Update signal
                    signal.mutate(|tabs| {
                        tabs.push(tab_data.clone());
                    });
                    if let Ok(mut shared_tabs) = shared.lock() {
                        *shared_tabs = signal.get().iter().cloned().collect();
                    }
                    self.validate_state();
                    Ok(())
                } else {
                    Err(TabsError::TabNotFound("History not enabled".to_string()))
                }
            },
        }
    }

    /// Clear tab history
    pub fn clear_history(&mut self) {
        self.tab_history = None;
    }

    /// Set the tab position
    pub fn with_position(mut self, position: TabPosition) -> Self {
        self.tab_position = position;
        self
    }

    /// Set the tab size
    pub fn with_tab_size(mut self, size: f32) -> Self {
        self.tab_size = size;
        self
    }

    /// Get the active tab index
    pub fn active_tab(&self) -> usize {
        *self.active_tab.get()
    }

    /// Set the active tab index
    pub fn set_active_tab(&mut self, index: usize) {
        let tabs_len = match &self.mode {
            TabsMode::Static => self.tabs.len(),
            TabsMode::Dynamic(signal, _, _) => signal.get().len(),
        };
        if index < tabs_len {
            self.active_tab.set(index);
        }
    }

    /// Validate and fix tab state consistency
    fn validate_state(&mut self) {
        let tabs_len = match &self.mode {
            TabsMode::Static => self.tabs.len(),
            TabsMode::Dynamic(signal, _, _) => signal.get().len(),
        };

        // Validate active tab index
        let current_active = *self.active_tab.get();
        if tabs_len == 0 {
            self.active_tab.set(0);
        } else if current_active >= tabs_len {
            // Active tab was removed, switch to nearest valid tab
            self.active_tab.set((tabs_len - 1).max(0));
        }

        // Validate hovered/pressed tab indices
        if let Some(hovered) = self.hovered_tab {
            if hovered >= tabs_len {
                self.hovered_tab = None;
            }
        }
        if let Some(pressed) = self.pressed_tab {
            if pressed >= tabs_len {
                self.pressed_tab = None;
            }
        }
        if let Some(close_hovered) = self.hovered_close {
            if close_hovered >= tabs_len {
                self.hovered_close = None;
            }
        }
        if let Some(close_pressed) = self.pressed_close {
            if close_pressed >= tabs_len {
                self.pressed_close = None;
            }
        }
        if let Some(dragging) = self.dragging_tab {
            if dragging >= tabs_len {
                self.dragging_tab = None;
            }
        }

        // Check for duplicate IDs (log warning)
        let ids: Vec<String> = match &self.mode {
            TabsMode::Static => self.tabs.iter().map(|t| t.id.clone()).collect(),
            TabsMode::Dynamic(signal, _, _) => signal.get().iter().map(|t| t.id.clone()).collect(),
        };
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        if ids.len() != unique_ids.len() {
            log::warn!("TabsContainer: Duplicate tab IDs detected");
        }
    }

    /// Ensure tab IDs are unique
    fn ensure_unique_ids(&self) -> Result<(), TabsError> {
        let ids: Vec<String> = match &self.mode {
            TabsMode::Static => self.tabs.iter().map(|t| t.id.clone()).collect(),
            TabsMode::Dynamic(signal, _, _) => signal.get().iter().map(|t| t.id.clone()).collect(),
        };
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        if ids.len() != unique_ids.len() {
            Err(TabsError::TabNotFound(
                "Duplicate tab IDs found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Rebuild tabs from signal (for dynamic mode)
    fn rebuild_tabs_from_signal(&mut self) {
        if let TabsMode::Dynamic(signal, shared, content_store) = &mut self.mode {
            // Read tab data from signal
            let signal_tabs_data: Vec<TabData> = signal.get().iter().cloned().collect();

            // Rebuild tabs by matching TabData with stored content widgets
            let mut new_tabs = Vec::new();
            let mut remaining_content: Vec<TabItem> = content_store.drain(..).collect();

            for tab_data in signal_tabs_data {
                // Find matching content widget by ID
                if let Some(pos) = remaining_content.iter().position(|t| t.id == tab_data.id) {
                    let mut tab = remaining_content.remove(pos);
                    // Update tab data
                    tab.label = tab_data.label.clone();
                    tab.enabled = tab_data.enabled;

                    // Always set a close callback for dynamic mode tabs to ensure
                    // they can be removed properly via Arc<Mutex>
                    let tab_id = tab.id.clone();
                    let shared_clone = shared.clone();

                    tab.on_close = Some(Arc::new(move || {
                        // Remove from shared - will be synced to signal in update()
                        // We use Arc<Mutex> to avoid RefCell borrow issues
                        if let Ok(mut shared_tabs) = shared_clone.lock() {
                            shared_tabs.retain(|t| t.id != tab_id);
                        }
                        // Schedule update via EVAL - the update() method will sync
                        Update::EVAL | Update::LAYOUT | Update::DRAW
                    }));
                    new_tabs.push(tab);
                }
                // If tab_data exists in signal but no content widget is found,
                // create a placeholder TabItem with default content
                else {
                    let tab_id = tab_data.id.clone();
                    let shared_clone = shared.clone();

                    let placeholder_content = Text::new(format!("Content for {}", tab_data.label));
                    let mut placeholder_tab =
                        TabItem::new(tab_id.clone(), tab_data.label.clone(), placeholder_content);
                    placeholder_tab.enabled = tab_data.enabled;

                    // Set close callback
                    placeholder_tab.on_close = Some(Arc::new(move || {
                        if let Ok(mut shared_tabs) = shared_clone.lock() {
                            shared_tabs.retain(|t| t.id != tab_id);
                        }
                        Update::EVAL | Update::LAYOUT | Update::DRAW
                    }));

                    new_tabs.push(placeholder_tab);
                }
            }
            // Update content store with rebuilt tabs (move, don't clone)
            *content_store = new_tabs;
            // In dynamic mode, we use content_store directly for rendering,
            // so we don't need to populate self.tabs
        }
    }

    /// Get mutable reference to tabs (for internal use)
    /// Note: In dynamic mode, ensure rebuild_tabs_from_signal() is called before this
    fn get_tabs_mut(&mut self) -> &mut Vec<TabItem> {
        match &mut self.mode {
            TabsMode::Static => &mut self.tabs,
            TabsMode::Dynamic(_, _, content_store) => {
                // In dynamic mode, return reference to content_store
                // Caller must ensure rebuild_tabs_from_signal() was called first
                content_store
            },
        }
    }

    /// Get current tabs (clones from Vec or rebuilds from signal)
    fn get_tabs(&self) -> Vec<TabItem> {
        match &self.mode {
            TabsMode::Static => {
                // Can't clone TabItem, so we need to reconstruct
                // For static mode, we can return a reference or reconstruct
                // Since we can't return &Vec, we'll need to handle this differently
                // For now, return empty and let render/update use self.tabs directly
                Vec::new()
            },
            TabsMode::Dynamic(_, _, _) => {
                // Return empty - tabs are accessed via content_store in dynamic mode
                Vec::new()
            },
        }
    }

    /// Calculate the width of a tab based on its label text
    fn calculate_tab_width(&self, label: &str, info: &mut AppInfo) -> f32 {
        let font_size = 14.0;
        let padding = 20.0; // Left + right padding

        // Use TextRenderContext to measure text width
        let text_width =
            self.text_render_context
                .measure_text_width(&mut info.font_context, label, None, font_size);

        let total_width = text_width + padding;

        // Add space for close button if tab has one
        // We'll check this when rendering, but for now assume no close button
        // You could pass the tab item here to check `on_close.is_some()`

        total_width.max(80.0) // Minimum tab width
    }

    /// Calculate total width needed for all tabs
    fn calculate_total_tabs_width(&self, info: &mut AppInfo) -> f32 {
        let tabs = match &self.mode {
            TabsMode::Static => &self.tabs,
            TabsMode::Dynamic(_, _, content_store) => content_store,
        };
        let tabs_width: f32 = tabs
            .iter()
            .map(|tab| {
                let base_width = self.calculate_tab_width(&tab.label, info);
                if tab.on_close.is_some() {
                    base_width + 20.0 // Add space for close button
                } else {
                    base_width
                }
            })
            .sum();

        // Add action button width if present
        let action_button_width = if self.action_button_callback.is_some() {
            self.action_button_size
        } else {
            0.0
        };

        tabs_width + action_button_width
    }

    /// Get action button bounds
    fn get_action_button_bounds(&self, layout: &LayoutNode, info: &mut AppInfo) -> Rect {
        let tab_bar_bounds = self.get_tab_bar_bounds(layout);
        let tabs = match &self.mode {
            TabsMode::Static => &self.tabs,
            TabsMode::Dynamic(_, _, content_store) => content_store,
        };

        match self.tab_position {
            TabPosition::Top | TabPosition::Bottom => {
                // Horizontal: button at the end after all tabs
                let mut total_tabs_width = 0.0f64;
                for tab in tabs.iter() {
                    let tab_width = self.calculate_tab_width(&tab.label, info) as f64;
                    let width_with_close = if tab.on_close.is_some() {
                        tab_width + 20.0
                    } else {
                        tab_width
                    };
                    total_tabs_width += width_with_close;
                }
                // Account for scroll offset
                let button_x = tab_bar_bounds.x0 + total_tabs_width - self.scroll_offset as f64;
                Rect::new(
                    button_x,
                    tab_bar_bounds.y0,
                    button_x + self.action_button_size as f64,
                    tab_bar_bounds.y1,
                )
            },
            TabPosition::Left | TabPosition::Right => {
                // Vertical: button at the bottom after all tabs
                let tab_height = if tabs.is_empty() {
                    0.0
                } else {
                    (tab_bar_bounds.height() - self.action_button_size as f64) / tabs.len() as f64
                };
                let total_tabs_height = tab_height * tabs.len() as f64;
                let button_y = tab_bar_bounds.y0 + total_tabs_height;
                Rect::new(
                    tab_bar_bounds.x0,
                    button_y,
                    tab_bar_bounds.x1,
                    button_y + self.action_button_size as f64,
                )
            },
        }
    }

    /// Get tab bounds for the given index within the tab bar area
    fn get_tab_bounds(&self, layout: &LayoutNode, index: usize, info: &mut AppInfo) -> Rect {
        let tab_bar_bounds = self.get_tab_bar_bounds(layout);
        let tabs = match &self.mode {
            TabsMode::Static => &self.tabs,
            TabsMode::Dynamic(_, _, content_store) => content_store,
        };
        let tab_count = tabs.len();

        if tab_count == 0 || index >= tab_count {
            return Rect::ZERO;
        }

        match self.tab_position {
            TabPosition::Top | TabPosition::Bottom => {
                // Horizontal tabs - use intrinsic widths + scrolling
                let mut current_x = tab_bar_bounds.x0 - self.scroll_offset as f64;

                for i in 0..index {
                    if let Some(tab) = tabs.get(i) {
                        let tab_width = self.calculate_tab_width(&tab.label, info) as f64;
                        let width_with_close = if tab.on_close.is_some() {
                            tab_width + 20.0
                        } else {
                            tab_width
                        };
                        current_x += width_with_close;
                    }
                }

                if let Some(tab) = tabs.get(index) {
                    let tab_width = self.calculate_tab_width(&tab.label, info) as f64;
                    let width_with_close = if tab.on_close.is_some() {
                        tab_width + 20.0
                    } else {
                        tab_width
                    };

                    Rect::new(
                        current_x,
                        tab_bar_bounds.y0,
                        current_x + width_with_close,
                        tab_bar_bounds.y1,
                    )
                } else {
                    Rect::ZERO
                }
            },
            TabPosition::Left | TabPosition::Right => {
                // Vertical tabs - distribute evenly across tab bar height
                let tab_height = tab_bar_bounds.height() / tab_count as f64;
                let tab_y = tab_bar_bounds.y0 + (index as f64 * tab_height);

                Rect::new(
                    tab_bar_bounds.x0,
                    tab_y,
                    tab_bar_bounds.x1,
                    tab_y + tab_height,
                )
            },
        }
    }

    /// Get content area bounds (excludes tab bar area)
    fn get_content_bounds(&self, layout: &LayoutNode) -> Rect {
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        match self.tab_position {
            TabPosition::Top => Rect::new(
                container_bounds.x0,
                container_bounds.y0 + self.tab_size as f64,
                container_bounds.x1,
                container_bounds.y1,
            ),
            TabPosition::Bottom => Rect::new(
                container_bounds.x0,
                container_bounds.y0,
                container_bounds.x1,
                container_bounds.y1 - self.tab_size as f64,
            ),
            TabPosition::Left => Rect::new(
                container_bounds.x0 + self.tab_size as f64,
                container_bounds.y0,
                container_bounds.x1,
                container_bounds.y1,
            ),
            TabPosition::Right => Rect::new(
                container_bounds.x0,
                container_bounds.y0,
                container_bounds.x1 - self.tab_size as f64,
                container_bounds.y1,
            ),
        }
    }

    /// Get tab bar area bounds (where the tabs themselves are rendered)
    fn get_tab_bar_bounds(&self, layout: &LayoutNode) -> Rect {
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        match self.tab_position {
            TabPosition::Top => Rect::new(
                container_bounds.x0,
                container_bounds.y0,
                container_bounds.x1,
                container_bounds.y0 + self.tab_size as f64,
            ),
            TabPosition::Bottom => Rect::new(
                container_bounds.x0,
                container_bounds.y1 - self.tab_size as f64,
                container_bounds.x1,
                container_bounds.y1,
            ),
            TabPosition::Left => Rect::new(
                container_bounds.x0,
                container_bounds.y0,
                container_bounds.x0 + self.tab_size as f64,
                container_bounds.y1,
            ),
            TabPosition::Right => Rect::new(
                container_bounds.x1 - self.tab_size as f64,
                container_bounds.y0,
                container_bounds.x1,
                container_bounds.y1,
            ),
        }
    }

    /// Get close button bounds for a tab
    fn get_close_button_bounds(&self, tab_bounds: Rect) -> Rect {
        let close_size = 16.0;
        let padding = 4.0;

        Rect::new(
            tab_bounds.x1 - close_size - padding,
            tab_bounds.y0 + (tab_bounds.height() - close_size) / 2.0,
            tab_bounds.x1 - padding,
            tab_bounds.y0 + (tab_bounds.height() + close_size) / 2.0,
        )
    }

    fn draw_close_button(
        &self,
        graphics: &mut dyn Graphics,
        close_bounds: Rect,
        is_hovered: bool,
        is_pressed: bool,
    ) {
        // SVG path data for the close button (X shape with rounded corners)
        // Create SVG with appropriate fill color based on state
        let (fill_color, opacity) = if is_pressed {
            // Pressed: #da4453 with 0.8 opacity
            ("#da4453", "0.8")
        } else if is_hovered {
            // Focused/hovered: #da4453
            ("#da4453", "1")
        } else {
            // Normal: #aaaaac with 0.97058835 opacity
            ("#aaaaac", "0.97058835")
        };

        // Normalize the path coordinates to start at (0, 0) by adjusting the viewBox
        // Original viewBox: "265 970 18 18", path starts at ~(268.75, 972)
        // Normalized: viewBox "0 0 18 18", path offset by (-265, -970)
        let svg_path = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18">
            <path d="m 3.7501,2 a 1.74961,1.74961 0 0 0 -1.21879,3.00483 l 3.9958,3.9935 -3.9958,3.9958 a 1.74961,1.74961 0 1 0 2.47403,2.47403 l 3.9958,-3.9958 3.9935,3.9958 a 1.74961,1.74961 0 1 0 2.47403,-2.47403 l -3.9935,-3.9958 3.9935,-3.9935 A 1.74961,1.74961 0 0 0 14.24988,2 a 1.74961,1.74961 0 0 0 -1.25524,0.5308 l -3.9935,3.9935 -3.9958,-3.9935 A 1.74961,1.74961 0 0 0 3.7501,2 Z" 
               fill="{}" fill-opacity="{}" stroke-width="3" stroke-linecap="round"/>
        </svg>"#,
            fill_color, opacity
        );

        // Parse SVG
        use vello_svg::usvg::{ImageRendering, Options, ShapeRendering, TextRendering, Tree};
        let tree = match Tree::from_str(
            &svg_path,
            &Options {
                shape_rendering: ShapeRendering::GeometricPrecision,
                text_rendering: TextRendering::OptimizeLegibility,
                image_rendering: ImageRendering::OptimizeSpeed,
                ..Default::default()
            },
        ) {
            Ok(tree) => tree,
            Err(_) => return,
        };

        // Render the SVG scene
        let scene = vello_svg::render_tree(&tree);

        // Calculate transform to fit the close button bounds
        let svg_size = tree.size();
        let svg_width = svg_size.width() as f64; // Should be 18.0
        let svg_height = svg_size.height() as f64; // Should be 18.0

        // Target icon size (should fit within close_bounds, which is 16x16)
        let target_size = close_bounds.width().min(close_bounds.height());
        let scale = target_size / svg_width.max(svg_height);

        // Calculate scaled dimensions
        let scaled_width = svg_width * scale;
        let scaled_height = svg_height * scale;

        // Right-align inside close_bounds
        let x = close_bounds.x1 - scaled_width;
        let y = close_bounds.y0 + (close_bounds.height() - scaled_height) / 2.0; // center vertically

        // Follow file_icon.rs pattern: scale first, then translate in screen space
        let transform = Affine::scale(scale).then_translate(Vec2::new(x, y));

        graphics.append(&scene, Some(transform));
    }

    fn draw_active_tab_accent(
        &self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        tab_bounds: Rect,
    ) {
        // Use a vibrant gradient color that will be visible regardless of tab background
        // Pink -> Purple -> Blue gradient like in the screenshot
        let band_thickness = ACCENT_THICKNESS;

        let make_gradient = |start: Point, end: Point| {
            Brush::Gradient(Gradient::new_linear(start, end).with_stops([
                (0.0, Color::from_rgb8(255, 0, 204)), // Bright pink
                // (0.5, Color::from_rgb8(186, 120, 255)), // Purple
                (1.0198, Color::from_rgb8(51, 51, 153)), // Blue
            ]))
        };

        match self.tab_position {
            TabPosition::Top => {
                // Draw gradient at bottom edge of tab (facing content)
                let grad_rect = Rect::new(
                    (tab_bounds.x0 + ACCENT_INSET).min(tab_bounds.x1),
                    (tab_bounds.y1 - band_thickness).max(tab_bounds.y0),
                    (tab_bounds.x1 - ACCENT_INSET).max(tab_bounds.x0),
                    tab_bounds.y1,
                );
                if grad_rect.width() > 0.0 && grad_rect.height() > 0.0 {
                    let brush = make_gradient(
                        Point::new(grad_rect.x0, grad_rect.y1),
                        Point::new(grad_rect.x1, grad_rect.y1),
                    );
                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &brush,
                        None,
                        &grad_rect.to_path(0.1),
                    );
                }
            },
            TabPosition::Bottom => {
                // Draw gradient at top edge of tab (facing content)
                let grad_rect = Rect::new(
                    (tab_bounds.x0 + ACCENT_INSET).min(tab_bounds.x1),
                    tab_bounds.y0,
                    (tab_bounds.x1 - ACCENT_INSET).max(tab_bounds.x0),
                    (tab_bounds.y0 + band_thickness).min(tab_bounds.y1),
                );
                if grad_rect.width() > 0.0 && grad_rect.height() > 0.0 {
                    let brush = make_gradient(
                        Point::new(grad_rect.x0, grad_rect.y0),
                        Point::new(grad_rect.x1, grad_rect.y0),
                    );
                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &brush,
                        None,
                        &grad_rect.to_path(0.1),
                    );
                }
            },
            TabPosition::Left => {
                // Draw gradient at right edge of tab (facing content)
                let grad_rect = Rect::new(
                    (tab_bounds.x1 - band_thickness).max(tab_bounds.x0),
                    (tab_bounds.y0 + ACCENT_INSET).min(tab_bounds.y1),
                    tab_bounds.x1,
                    (tab_bounds.y1 - ACCENT_INSET).max(tab_bounds.y0),
                );
                if grad_rect.width() > 0.0 && grad_rect.height() > 0.0 {
                    let brush = make_gradient(
                        Point::new(grad_rect.x1, grad_rect.y0),
                        Point::new(grad_rect.x1, grad_rect.y1),
                    );
                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &brush,
                        None,
                        &grad_rect.to_path(0.1),
                    );
                }
            },
            TabPosition::Right => {
                // Draw gradient at left edge of tab (facing content)
                let grad_rect = Rect::new(
                    tab_bounds.x0,
                    (tab_bounds.y0 + ACCENT_INSET).min(tab_bounds.y1),
                    (tab_bounds.x0 + band_thickness).min(tab_bounds.x1),
                    (tab_bounds.y1 - ACCENT_INSET).max(tab_bounds.y0),
                );
                if grad_rect.width() > 0.0 && grad_rect.height() > 0.0 {
                    let brush = make_gradient(
                        Point::new(grad_rect.x0, grad_rect.y0),
                        Point::new(grad_rect.x0, grad_rect.y1),
                    );
                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &brush,
                        None,
                        &grad_rect.to_path(0.1),
                    );
                }
            },
        }
    }

    /// Render text on a tab
    fn render_text(
        text_render_context: &mut TextRenderContext,
        graphics: &mut dyn Graphics,
        text: &str,
        x: f64,
        y: f64,
        color: Color,
        info: &mut AppInfo,
    ) {
        let font_size = 14.0;

        if text.is_empty() {
            return;
        }

        let transform = Affine::translate((x, y));

        text_render_context.render_text(
            &mut info.font_context,
            graphics,
            text,
            None, // No specific font, use default
            font_size,
            Brush::Solid(color),
            transform,
            true, // hinting
            None, // No width constraint for tab labels
        );
    }
}

impl Widget for TabsContainer {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // If dynamic mode, rebuild tabs from signal
        if matches!(self.mode, TabsMode::Dynamic(_, _, _)) {
            self.rebuild_tabs_from_signal();
        }

        // Get tabs based on mode
        let tabs = match &self.mode {
            TabsMode::Static => &self.tabs,
            TabsMode::Dynamic(_, _, content_store) => content_store,
        };

        // Draw container background
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        if let Some(bg_color) = theme.get_property(
            self.widget_id(),
            &nptk_theme::properties::ThemeProperty::ColorBackground,
        ) {
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(bg_color),
                None,
                &container_bounds.to_path(0.1),
            );
        }

        // Draw tab bar background (slightly different color)
        let tab_bar_bounds = self.get_tab_bar_bounds(layout);
        let tab_bar_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::TabBarBackground,
            )
            .unwrap_or_else(|| Color::from_rgb8(255, 255, 255));

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(tab_bar_color),
            None,
            &tab_bar_bounds.to_path(0.1),
        );

        // Draw tab bar border
        let border_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorBorder,
            )
            .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)); // Default border color

        graphics.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(border_color),
            None,
            &tab_bar_bounds.to_path(0.1),
        );

        // Draw tabs
        for (index, tab) in tabs.iter().enumerate() {
            let tab_bounds = self.get_tab_bounds(layout, index, info);
            let is_active = index == self.active_tab();
            let is_hovered = self.hovered_tab == Some(index);
            let is_pressed = self.pressed_tab == Some(index);

            // Tab background with rounded corners
            let tab_color = if is_active {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabActive,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(255, 255, 255))
            } else if is_pressed {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabPressed,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(100, 150, 255))
            } else if is_hovered {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabHovered,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180))
            } else {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabInactive,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(255, 255, 255))
            };

            // Create rounded rectangle for tab (only round top corners for top tabs)
            let tab_rounded = match self.tab_position {
                TabPosition::Top => RoundedRect::from_rect(
                    tab_bounds,
                    RoundedRectRadii::new(TAB_CORNER_RADIUS, TAB_CORNER_RADIUS, 0.0, 0.0),
                ),
                TabPosition::Bottom => RoundedRect::from_rect(
                    tab_bounds,
                    RoundedRectRadii::new(0.0, 0.0, TAB_CORNER_RADIUS, TAB_CORNER_RADIUS),
                ),
                TabPosition::Left => RoundedRect::from_rect(
                    tab_bounds,
                    RoundedRectRadii::new(TAB_CORNER_RADIUS, 0.0, 0.0, TAB_CORNER_RADIUS),
                ),
                TabPosition::Right => RoundedRect::from_rect(
                    tab_bounds,
                    RoundedRectRadii::new(0.0, TAB_CORNER_RADIUS, TAB_CORNER_RADIUS, 0.0),
                ),
            };

            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(tab_color),
                None,
                &tab_rounded.to_path(0.1),
            );

            // Tab border with subtle styling
            let border_color = theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBorder,
                )
                .unwrap_or_else(|| Color::from_rgb8(200, 200, 200)); // Default border color

            graphics.stroke(
                &Stroke::new(if is_active { 1.5 } else { 1.0 }),
                Affine::IDENTITY,
                &Brush::Solid(border_color),
                None,
                &tab_rounded.to_path(0.1),
            );

            if is_active {
                self.draw_active_tab_accent(graphics, theme, tab_bounds);
            }

            // Tab text with proper rendering and theme colors
            let text_color = if is_active {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabTextActive,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(0, 0, 0))
            } else {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabText,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(0, 0, 0))
            };

            // Center text in tab
            let text_x = tab_bounds.x0 + 10.0; // Left padding
            let text_y = tab_bounds.y0 + (tab_bounds.height() - 14.0) / 2.0; // Center vertically

            Self::render_text(
                &mut self.text_render_context,
                graphics,
                &tab.label,
                text_x,
                text_y,
                text_color,
                info,
            );

            // Close button if available
            if tab.on_close.is_some() {
                let close_bounds = self.get_close_button_bounds(tab_bounds);
                let close_hovered = self.hovered_close == Some(index);
                let close_pressed = self.pressed_close == Some(index);

                self.draw_close_button(graphics, close_bounds, close_hovered, close_pressed);
            }
        }

        // Render action button if present
        if let Some(_callback) = &self.action_button_callback {
            let action_bounds = self.get_action_button_bounds(layout, info);

            // Button background
            let button_color = if self.action_button_hovered {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabHovered,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180))
            } else {
                theme
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::TabInactive,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(240, 240, 240))
            };

            // Create rounded rectangle for button
            let button_rounded = match self.tab_position {
                TabPosition::Top => {
                    RoundedRect::from_rect(action_bounds, RoundedRectRadii::new(6.0, 6.0, 0.0, 0.0))
                },
                TabPosition::Bottom => {
                    RoundedRect::from_rect(action_bounds, RoundedRectRadii::new(0.0, 0.0, 6.0, 6.0))
                },
                TabPosition::Left => {
                    RoundedRect::from_rect(action_bounds, RoundedRectRadii::new(6.0, 0.0, 0.0, 6.0))
                },
                TabPosition::Right => {
                    RoundedRect::from_rect(action_bounds, RoundedRectRadii::new(0.0, 6.0, 6.0, 0.0))
                },
            };

            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(button_color),
                None,
                &button_rounded.to_path(0.1),
            );

            // Button border
            graphics.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(border_color),
                None,
                &button_rounded.to_path(0.1),
            );

            // Draw "+" symbol
            let center_x = action_bounds.center().x;
            let center_y = action_bounds.center().y;
            let plus_size = 12.0;
            let line_width = 2.0;

            // Horizontal line
            graphics.stroke(
                &Stroke::new(line_width),
                Affine::IDENTITY,
                &Brush::Solid(Color::from_rgb8(0, 0, 0)),
                None,
                &Rect::new(
                    center_x - plus_size / 2.0,
                    center_y - line_width / 2.0,
                    center_x + plus_size / 2.0,
                    center_y + line_width / 2.0,
                )
                .to_path(0.1),
            );

            // Vertical line
            graphics.stroke(
                &Stroke::new(line_width),
                Affine::IDENTITY,
                &Brush::Solid(Color::from_rgb8(0, 0, 0)),
                None,
                &Rect::new(
                    center_x - line_width / 2.0,
                    center_y - plus_size / 2.0,
                    center_x + line_width / 2.0,
                    center_y + plus_size / 2.0,
                )
                .to_path(0.1),
            );
        }

        // Render active tab content in the content area
        let active_tab_index = self.active_tab();
        let content_bounds = self.get_content_bounds(layout);
        let widget_id = self.widget_id();

        if let Some(active_tab) = self.tabs.get_mut(active_tab_index) {
            // Draw content area background
            let content_bg_color = theme
                .get_property(
                    widget_id,
                    &nptk_theme::properties::ThemeProperty::ContentBackground,
                )
                .unwrap_or_else(|| Color::from_rgb8(255, 255, 255));

            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(content_bg_color),
                None,
                &content_bounds.to_path(0.1),
            );

            // Draw content area border
            graphics.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(border_color),
                None,
                &content_bounds.to_path(0.1),
            );

            // Apply clipping to content area to prevent content from leaking outside bounds
            graphics.push_layer(
                Mix::Clip,
                1.0,
                Affine::IDENTITY,
                &content_bounds.to_path(0.1),
            );

            // Render content directly in the content area using child layout if available
            if !layout.children.is_empty() {
                // Use the first child's layout (which should be positioned correctly)
                active_tab
                    .content
                    .render(graphics, theme, &layout.children[0], info, context);
            } else {
                // Fallback: just render with original layout (content might overlap tabs)
                active_tab
                    .content
                    .render(graphics, theme, layout, info, context);
            }

            // Pop the clipping layer
            graphics.pop_layer();
        }
    }

    fn render_postfix(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Propagate render_postfix to active tab content (for overlays, popups, etc.)
        let active_tab_index = self.active_tab();
        let content_bounds = self.get_content_bounds(layout);

        if let Some(active_tab) = self.tabs.get_mut(active_tab_index) {
            // Apply same clipping as in render() to ensure overlays are properly clipped
            if !layout.children.is_empty() {
                // Apply clipping to content area
                graphics.push_layer(
                    Mix::Clip,
                    1.0,
                    Affine::IDENTITY,
                    &content_bounds.to_path(0.1),
                );

                // Propagate render_postfix to content
                active_tab.content.render_postfix(
                    graphics,
                    theme,
                    &layout.children[0],
                    info,
                    context,
                );

                // Pop the clipping layer
                graphics.pop_layer();
            } else {
                // Fallback: render with original layout (content might overlap tabs)
                active_tab.content.render_postfix(graphics, theme, layout, info, context);
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // If dynamic mode, sync signal changes from Arc<Mutex> to signal
        let needs_rebuild = if let TabsMode::Dynamic(signal, shared, _) = &self.mode {
            // Get signal data first (clone to avoid borrow issues)
            let signal_vec: Vec<TabData> = {
                let signal_ref = signal.get();
                signal_ref.iter().cloned().collect()
            };

            // Get shared data and compare
            let mut needs_rebuild = false;
            if let Ok(shared_tabs) = shared.lock() {
                let shared_vec: Vec<TabData> = shared_tabs.iter().cloned().collect();

                if shared_vec.len() != signal_vec.len()
                    || shared_vec
                        .iter()
                        .zip(signal_vec.iter())
                        .any(|(a, b)| a.id != b.id)
                {
                    // Sync from shared to signal (after releasing all borrows)
                    drop(shared_tabs);
                    signal.set(shared_vec);
                    needs_rebuild = true;
                }
            }
            needs_rebuild
        } else {
            false
        };

        if needs_rebuild {
            self.rebuild_tabs_from_signal();
            self.validate_state();
            update |= Update::DRAW | Update::LAYOUT;
        }

        // Calculate max scroll based on total tabs width vs available width
        let tab_bar_bounds = self.get_tab_bar_bounds(layout);
        let total_tabs_width = self.calculate_total_tabs_width(info);
        let available_width = tab_bar_bounds.width() as f32;
        self.max_scroll = (total_tabs_width - available_width).max(0.0);

        // Clamp scroll offset
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.max_scroll);

        // Update mouse position
        if let Some(cursor_pos) = info.cursor_pos {
            self.mouse_pos = Vector2::new(cursor_pos.x as f32, cursor_pos.y as f32);
        }

        // Check action button hover
        self.action_button_hovered = false;
        if let Some(ref _callback) = self.action_button_callback {
            let action_bounds = self.get_action_button_bounds(layout, info);
            if let Some(cursor_pos) = info.cursor_pos {
                if action_bounds.contains(Point::new(cursor_pos.x, cursor_pos.y)) {
                    self.action_button_hovered = true;
                }
            }
        }

        // Check tab hover states
        self.hovered_tab = None;
        self.hovered_close = None;
        // Note: pressed_close is only cleared on mouse release

        let tabs = match &self.mode {
            TabsMode::Static => &self.tabs,
            TabsMode::Dynamic(_, _, content_store) => content_store,
        };
        for (index, tab) in tabs.iter().enumerate() {
            let tab_bounds = self.get_tab_bounds(layout, index, info);

            if tab_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64)) {
                self.hovered_tab = Some(index);

                // Check close button hover if tab has close button
                if tab.on_close.is_some() {
                    let close_bounds = self.get_close_button_bounds(tab_bounds);
                    if close_bounds
                        .contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64))
                    {
                        self.hovered_close = Some(index);
                    }
                }
                break;
            }
        }

        // Handle mouse wheel scrolling (for horizontal tabs only)
        if matches!(self.tab_position, TabPosition::Top | TabPosition::Bottom) {
            for scroll_delta in &info.mouse_scroll_delta {
                match scroll_delta {
                    nptk_core::window::MouseScrollDelta::LineDelta(_, y) => {
                        // Scroll horizontally with vertical wheel
                        self.scroll_offset -= y * 30.0; // 30 pixels per line
                        self.scroll_offset = self.scroll_offset.clamp(0.0, self.max_scroll);
                        update |= Update::DRAW;
                    },
                    nptk_core::window::MouseScrollDelta::PixelDelta(delta) => {
                        self.scroll_offset -= delta.y as f32;
                        self.scroll_offset = self.scroll_offset.clamp(0.0, self.max_scroll);
                        update |= Update::DRAW;
                    },
                }
            }
        }

        // Handle tab dragging for reordering
        if let Some(cursor_pos) = info.cursor_pos {
            if let Some(dragging_index) = self.dragging_tab {
                // We're dragging a tab - check if we should reorder
                let mut target_index = dragging_index;

                // Find which tab position the mouse is over
                let tabs = match &self.mode {
                    TabsMode::Static => &self.tabs,
                    TabsMode::Dynamic(_, _, content_store) => content_store,
                };
                for (i, _) in tabs.iter().enumerate() {
                    let bounds = self.get_tab_bounds(layout, i, info);
                    if cursor_pos.x >= bounds.x0 as f64 && cursor_pos.x <= bounds.x1 as f64 {
                        target_index = i;
                        break;
                    }
                }

                // Reorder if different from current position
                if target_index != dragging_index {
                    match &mut self.mode {
                        TabsMode::Static => {
                            let tab = self.tabs.remove(dragging_index);
                            self.tabs.insert(target_index, tab);
                            self.dragging_tab = Some(target_index);
                            update |= Update::DRAW;
                        },
                        TabsMode::Dynamic(signal, shared, content_store) => {
                            // Reorder in signal
                            signal.mutate(|tabs| {
                                let tab = tabs.remove(dragging_index);
                                tabs.insert(target_index, tab);
                            });
                            // Reorder in content store
                            let tab = content_store.remove(dragging_index);
                            content_store.insert(target_index, tab);
                            // Sync to shared
                            if let Ok(mut shared_tabs) = shared.lock() {
                                *shared_tabs = signal.get().iter().cloned().collect();
                            }
                            self.dragging_tab = Some(target_index);
                            update |= Update::DRAW;
                        },
                    }
                }
            }
        }

        // Pre-calculate cursor position for drag offset calculation to avoid borrow issues
        let cursor_pos_for_drag = info.cursor_pos;

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left {
                match *state {
                    ElementState::Pressed => {
                        // Check action button click first
                        if self.action_button_hovered {
                            if let Some(ref callback) = self.action_button_callback {
                                update |= callback();
                            }
                        } else if let Some(hovered_tab) = self.hovered_tab {
                            self.pressed_tab = Some(hovered_tab);

                            // Check if clicking close button
                            if self.hovered_close == Some(hovered_tab) {
                                self.pressed_close = Some(hovered_tab);
                                update |= Update::DRAW;
                            } else {
                                // Start dragging for reorder - use pre-calculated bounds
                                if let Some(cursor_pos) = cursor_pos_for_drag {
                                    // Calculate drag offset manually to avoid borrowing issues
                                    let tab_bar_bounds = self.get_tab_bar_bounds(layout);
                                    let mut current_x =
                                        tab_bar_bounds.x0 - self.scroll_offset as f64;

                                    // Simple approximation - use fixed width to avoid mutable borrow
                                    let tab_width = 100.0f64; // Approximate width
                                    current_x += tab_width * hovered_tab as f64;

                                    self.dragging_tab = Some(hovered_tab);
                                    self.drag_offset = (cursor_pos.x - current_x) as f32;
                                }

                                // Switch to clicked tab
                                self.set_active_tab(hovered_tab);
                                update |= Update::DRAW;
                            }
                        }
                    },
                    ElementState::Released => {
                        // Check if releasing on close button
                        if let Some(pressed_close_idx) = self.pressed_close {
                            if self.hovered_close == Some(pressed_close_idx) {
                                let tabs = match &self.mode {
                                    TabsMode::Static => &self.tabs,
                                    TabsMode::Dynamic(_, _, content_store) => content_store,
                                };
                                if let Some(tab) = tabs.get(pressed_close_idx) {
                                    if let Some(ref callback) = tab.on_close {
                                        update |= callback();
                                    }
                                }
                            }
                        }
                        self.pressed_tab = None;
                        self.pressed_close = None;
                        self.dragging_tab = None;
                    },
                }
            }
        }

        // Update active tab content
        let active_tab_index = self.active_tab();
        let _content_bounds = self.get_content_bounds(layout);
        let tabs_mut = self.get_tabs_mut();
        if let Some(active_tab) = tabs_mut.get_mut(active_tab_index) {
            // Use the parent layout for content rendering (simplified approach)
            let content_layout = layout;

            update |= active_tab.content.update(&content_layout, context, info);
        }

        update
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: if let Some(active_tab) = self.tabs.get(self.active_tab()) {
                // Include the active tab's content as a child with adjusted position
                let mut child_style = active_tab.content.layout_style();

                // Adjust the child's position based on tab position
                use nptk_core::layout::LengthPercentageAuto;

                match self.tab_position {
                    TabPosition::Top => {
                        child_style.style.margin.top = LengthPercentageAuto::length(self.tab_size);
                    },
                    TabPosition::Bottom => {
                        child_style.style.margin.bottom =
                            LengthPercentageAuto::length(self.tab_size);
                    },
                    TabPosition::Left => {
                        child_style.style.margin.left = LengthPercentageAuto::length(self.tab_size);
                    },
                    TabPosition::Right => {
                        child_style.style.margin.right =
                            LengthPercentageAuto::length(self.tab_size);
                    },
                }

                vec![child_style]
            } else {
                vec![]
            },
        }
    }

    fn widget_id(&self) -> WidgetId {
        self.widget_id.clone()
    }
}

impl WidgetLayoutExt for TabsContainer {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }

    fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self
    where
        Self: Sized,
    {
        self.set_layout_style(layout_style);
        self
    }
}
