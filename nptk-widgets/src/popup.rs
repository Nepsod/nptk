// SPDX-License-Identifier: MIT OR Apache-2.0

//! Popup widget for displaying overlay content

use nalgebra::Vector2;

use nptk_core::widget::Widget;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::vg::{Scene};
use nptk_core::vg::kurbo::{Affine, Rect, Size, Stroke};
use nptk_core::vg::peniko::{Color, Fill};
use nptk_theme::theme::Theme;
use nptk_theme::id::WidgetId;
use nptk_core::app::info::AppInfo;
use nptk_core::app::context::AppContext;
use nptk_core::app::overlay_manager::OverlayId;
use nptk_core::app::update::Update;

/// A popup widget that can be positioned relative to other widgets
pub struct Popup {
    /// The content widget to display in the popup
    content: Box<dyn Widget>,
    /// The ID of the widget this popup is anchored to
    anchor_widget: Option<WidgetId>,
    /// The position of the popup relative to its anchor
    position: PopupPosition,
    /// Whether the popup is currently visible
    is_visible: bool,
    /// The overlay ID assigned by the overlay manager
    overlay_id: Option<OverlayId>,
    /// Background color of the popup
    background_color: Option<Color>,
    /// Border color of the popup
    border_color: Option<Color>,
    /// Border width of the popup
    border_width: f32,
    /// Padding around the content
    padding: f32,
}

/// Position of a popup relative to its anchor widget
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupPosition {
    /// Position above the anchor widget
    Above,
    /// Position below the anchor widget
    Below,
    /// Position to the left of the anchor widget
    Left,
    /// Position to the right of the anchor widget
    Right,
    /// Position at a specific point
    Absolute(Vector2<f64>),
    /// Position at the center of the screen
    Center,
}

impl Popup {
    /// Create a new popup with the given content
    pub fn new(content: Box<dyn Widget>) -> Self {
        Self {
            content,
            anchor_widget: None,
            position: PopupPosition::Below,
            is_visible: false,
            overlay_id: None,
            background_color: Some(Color::from_rgb8(240, 240, 240)),
            border_color: Some(Color::from_rgb8(200, 200, 200)),
            border_width: 1.0,
            padding: 8.0,
        }
    }

    /// Set the anchor widget for this popup
    pub fn with_anchor(mut self, anchor: WidgetId) -> Self {
        self.anchor_widget = Some(anchor);
        self
    }

    /// Set the position of the popup
    pub fn with_position(mut self, position: PopupPosition) -> Self {
        self.position = position;
        self
    }

    /// Set the background color
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Set the border color
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set the border width
    pub fn with_border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Set the padding around the content
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Show the popup
    pub fn show(&mut self, info: &mut AppInfo) {
        if !self.is_visible {
            self.is_visible = true;
            
            // Add to overlay manager
            let layout_node = LayoutNode {
                layout: nptk_core::layout::Layout::default(),
                children: Vec::new(),
            };
            
            // Create a simple tooltip as content for now
            let tooltip_content = Box::new(Tooltip::new("This is a popup!".to_string()));
            
            self.overlay_id = Some(info.overlay_manager.add_layer(
                nptk_core::app::overlay_manager::OverlayLevel::Popup,
                tooltip_content,
                layout_node,
            ));
            
            // Set the position using smart positioning
            if let Some(overlay_id) = self.overlay_id {
                let position = self.calculate_position(None, Size::new(800.0, 600.0)); // TODO: Get actual screen size
                let anchor_bounds = None; // TODO: Get actual anchor bounds
                
                info.overlay_manager.set_layer_position(overlay_id, position, anchor_bounds);
                info.overlay_manager.set_layer_size(overlay_id, Vector2::new(200.0, 100.0));
            }
        }
    }

    /// Hide the popup
    pub fn hide(&mut self, info: &mut AppInfo) {
        if self.is_visible {
            self.is_visible = false;
            
            if let Some(overlay_id) = self.overlay_id {
                info.overlay_manager.remove_overlay(overlay_id);
                self.overlay_id = None;
            }
        }
    }

    /// Toggle the popup visibility
    pub fn toggle(&mut self, info: &mut AppInfo) {
        if self.is_visible {
            self.hide(info);
        } else {
            self.show(info);
        }
    }

    /// Check if the popup is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Calculate the position of the popup based on its anchor and position setting
    fn calculate_position(&self, anchor_bounds: Option<Rect>, screen_size: Size) -> Vector2<f64> {
        match self.position {
            PopupPosition::Above => {
                if let Some(bounds) = anchor_bounds {
                    Vector2::new(bounds.min_x(), bounds.min_y() - 10.0) // 10px gap
                } else {
                    Vector2::new(0.0, 0.0)
                }
            }
            PopupPosition::Below => {
                if let Some(bounds) = anchor_bounds {
                    Vector2::new(bounds.min_x(), bounds.max_y() + 10.0) // 10px gap
                } else {
                    Vector2::new(0.0, 0.0)
                }
            }
            PopupPosition::Left => {
                if let Some(bounds) = anchor_bounds {
                    Vector2::new(bounds.min_x() - 10.0, bounds.min_y()) // 10px gap
                } else {
                    Vector2::new(0.0, 0.0)
                }
            }
            PopupPosition::Right => {
                if let Some(bounds) = anchor_bounds {
                    Vector2::new(bounds.max_x() + 10.0, bounds.min_y()) // 10px gap
                } else {
                    Vector2::new(0.0, 0.0)
                }
            }
            PopupPosition::Absolute(pos) => pos,
            PopupPosition::Center => {
                Vector2::new(
                    (screen_size.width - 200.0) / 2.0, // Assume 200px width
                    (screen_size.height - 100.0) / 2.0, // Assume 100px height
                )
            }
        }
    }
}

impl Widget for Popup {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk_widgets", "popup")
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: LayoutStyle::default(),
            children: Vec::new(),
        }
    }

    fn render(
        &mut self,
        _scene: &mut Scene,
        _theme: &mut dyn Theme,
        _layout_node: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Popups are rendered through the overlay manager, not here
    }

    fn update(
        &mut self,
        _layout: &LayoutNode,
        _context: AppContext,
        _info: &mut AppInfo,
    ) -> Update {
        // Handle popup-specific updates if needed
        Update::empty()
    }
}

/// A simple tooltip widget that can be used as popup content
pub struct Tooltip {
    text: String,
    max_width: f32,
}

impl Tooltip {
    /// Create a new tooltip with the given text
    pub fn new(text: String) -> Self {
        Self {
            text,
            max_width: 200.0,
        }
    }

    /// Set the maximum width of the tooltip
    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }
}

impl Widget for Tooltip {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk_widgets", "tooltip")
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: LayoutStyle::default(),
            children: Vec::new(),
        }
    }

    fn render(
        &mut self,
        scene: &mut Scene,
        _theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        let bounds = Rect::new(
            layout_node.layout.location.x as f64,
            layout_node.layout.location.y as f64,
            (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
            (layout_node.layout.location.y + layout_node.layout.size.height) as f64,
        );

        // Draw background
        let bg_color = Color::from_rgba8(50, 50, 50, 200);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            bg_color,
            None,
            &bounds,
        );

        // Draw border
        let border_color = Color::from_rgb8(100, 100, 100);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            border_color,
            None,
            &bounds,
        );

        // TODO: Draw text using TextRenderContext
        // For now, we'll just draw the background and border
    }

    fn update(
        &mut self,
        _layout: &LayoutNode,
        _context: AppContext,
        _info: &mut AppInfo,
    ) -> Update {
        // Tooltips don't need updates
        Update::empty()
    }
}
