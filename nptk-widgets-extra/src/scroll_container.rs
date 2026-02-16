// SPDX-License-Identifier: LGPL-3.0-only
//! ScrollContainer widget with enhanced layout flexibility.
//!
//! This widget provides scrolling capabilities with support for:
//! - Measure functions for accurate content sizing
//! - Constraint-aware scrolling that adapts to parent constraints
//! - Flexible scrollbar space reservation
//! - Responsive scrollbar behavior
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, LengthPercentage, StyleNode, LayoutContext, OverflowDetector, ViewportBounds};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::vg::kurbo::{
    Affine, BezPath, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke,
};
use nptk_core::vg::peniko::{Brush, Fill, Mix};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton, MouseScrollDelta};
use nptk_core::theme::{ColorRole, Palette};
use async_trait::async_trait;

/// Determines when scrollbars should be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalScrollbarPosition {
    /// Position scrollbar on the left side
    Left,
    /// Position scrollbar on the right side
    Right,
}

impl Default for VerticalScrollbarPosition {
    fn default() -> Self {
        Self::Left
    }
}

/// Scrollbar buttons visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarButtons {
    /// Always show scrollbar buttons
    Always,
    /// Never show scrollbar buttons
    Never,
}

impl Default for ScrollbarButtons {
    fn default() -> Self {
        Self::Always
    }
}

/// Scrolling direction for the scroll container
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Only vertical scrolling
    Vertical,
    /// Only horizontal scrolling  
    Horizontal,
    /// Both vertical and horizontal scrolling
    Both,
    /// No scrolling (content is clipped)
    None,
}

/// Scrollbar visibility behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarVisibility {
    /// Always show scrollbars
    Always,
    /// Show scrollbars only when needed
    Auto,
    /// Never show scrollbars
    Never,
}

/// State for scrollbar interaction (dragging and hover)
struct ScrollbarState {
    dragging_vertical: bool,
    dragging_horizontal: bool,
    drag_start_pos: Vector2<f32>,
    drag_start_offset: Vector2<f32>,
    vertical_scrollbar_hovered: bool,
    horizontal_scrollbar_hovered: bool,
    prev_dragging_vertical: bool,
    prev_dragging_horizontal: bool,
}

impl Default for ScrollbarState {
    fn default() -> Self {
        Self {
            dragging_vertical: false,
            dragging_horizontal: false,
            drag_start_pos: Vector2::new(0.0, 0.0),
            drag_start_offset: Vector2::new(0.0, 0.0),
            vertical_scrollbar_hovered: false,
            horizontal_scrollbar_hovered: false,
            prev_dragging_vertical: false,
            prev_dragging_horizontal: false,
        }
    }
}

/// State for scrollbar button interaction
struct ScrollbarButtonState {
    up_button_hovered: bool,
    down_button_hovered: bool,
    left_button_hovered: bool,
    right_button_hovered: bool,
    button_held: Option<ArrowDirection>,
    prev_button_held: Option<ArrowDirection>,
    prev_up_button_hovered: bool,
    prev_down_button_hovered: bool,
    prev_left_button_hovered: bool,
    prev_right_button_hovered: bool,
}

impl Default for ScrollbarButtonState {
    fn default() -> Self {
        Self {
            up_button_hovered: false,
            down_button_hovered: false,
            left_button_hovered: false,
            right_button_hovered: false,
            button_held: None,
            prev_button_held: None,
            prev_up_button_hovered: false,
            prev_down_button_hovered: false,
            prev_left_button_hovered: false,
            prev_right_button_hovered: false,
        }
    }
}

/// State for scrollbar visibility calculations in layout
struct ScrollbarVisibilityState {
    prev_needs_vertical_scrollbar: bool,
    prev_needs_horizontal_scrollbar: bool,
}

impl Default for ScrollbarVisibilityState {
    fn default() -> Self {
        Self {
            prev_needs_vertical_scrollbar: false,
            prev_needs_horizontal_scrollbar: false,
        }
    }
}

/// A container widget that provides scrolling functionality for its content
///
/// ### Theming
/// Styling the scroll container requires the following properties:
/// - `color_background` - The background color of the scroll area.
/// - `color_scrollbar` - The color of the scrollbar track.
/// - `color_scrollbar_thumb` - The color of the scrollbar thumb.
/// - `color_scrollbar_thumb_hover` - The color of the scrollbar thumb when hovered.
/// - `color_scrollbar_thumb_active` - The color of the scrollbar thumb when dragged.
/// - `color_border` - The border color of the container.
pub struct ScrollContainer {
    child: Option<BoxedWidget>,
    layout_style: MaybeSignal<LayoutStyle>,

    // Scroll configuration
    scroll_direction: ScrollDirection,
    scrollbar_visibility: ScrollbarVisibility,
    scrollbar_width: f32,
    vertical_scrollbar_position: VerticalScrollbarPosition,
    scrollbar_buttons: ScrollbarButtons,
    button_size: f32,

    // Scroll state - make this reactive with MaybeSignal
    scroll_offset: StateSignal<Vector2<f32>>,
    // Last known scroll offset to detect changes in update()
    _last_scroll_offset: Vector2<f32>,
    content_size: Vector2<f32>,
    viewport_size: Vector2<f32>,

    // Mouse state
    mouse_pos: Vector2<f32>,

    // Scrollbar interaction state
    scrollbar_state: ScrollbarState,
    button_state: ScrollbarButtonState,
    visibility_state: ScrollbarVisibilityState,

    // Virtual scrolling (for performance with large lists)
    virtual_scrolling: bool,
    item_height: f32,
    visible_range: (usize, usize),
}

impl ScrollContainer {
    /// Create a new scroll container
    pub fn new() -> Self {
        Self {
            child: None,
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                overflow: (
                    nptk_core::layout::Overflow::Hidden,
                    nptk_core::layout::Overflow::Hidden,
                ),
                ..Default::default()
            }
            .into(),
            scroll_direction: ScrollDirection::Both,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            scrollbar_width: 12.0,
            vertical_scrollbar_position: VerticalScrollbarPosition::default(),
            scrollbar_buttons: ScrollbarButtons::default(),
            button_size: 16.0,
            scroll_offset: StateSignal::new(Vector2::new(0.0, 0.0)),
            _last_scroll_offset: Vector2::new(0.0, 0.0),
            content_size: Vector2::new(0.0, 0.0),
            viewport_size: Vector2::new(0.0, 0.0),
            mouse_pos: Vector2::new(0.0, 0.0),
            scrollbar_state: ScrollbarState::default(),
            button_state: ScrollbarButtonState::default(),
            visibility_state: ScrollbarVisibilityState::default(),
            virtual_scrolling: false,
            item_height: 30.0,
            visible_range: (0, 0),
        }
    }

    /// Set the child widget to scroll
    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    /// Initialize reactive scroll layout (should be called with AppContext)
    pub fn init_reactive_scroll(&mut self, context: &AppContext) {
        // Hook our internal StateSignal into the AppContext so changes become reactive
        // This will make signal changes insert Update::EVAL which in turn allows
        // widgets to request LAYOUT/DRAW updates in their update() implementation.
        context.hook_signal(&mut self.scroll_offset);
    }

    /// Set the scroll direction
    pub fn with_scroll_direction(mut self, direction: ScrollDirection) -> Self {
        self.scroll_direction = direction;
        self
    }

    /// Set the scrollbar visibility behavior
    pub fn with_scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    /// Set the scrollbar width
    pub fn with_scrollbar_width(mut self, width: f32) -> Self {
        self.scrollbar_width = width;
        self
    }

    /// Set the vertical scrollbar position
    pub fn with_vertical_scrollbar_position(mut self, position: VerticalScrollbarPosition) -> Self {
        self.vertical_scrollbar_position = position;
        self
    }

    /// Set the scrollbar buttons visibility
    pub fn with_scrollbar_buttons(mut self, visibility: ScrollbarButtons) -> Self {
        self.scrollbar_buttons = visibility;
        self
    }

    /// Enable virtual scrolling for performance with large lists
    pub fn with_virtual_scrolling(mut self, enabled: bool, item_height: f32) -> Self {
        self.virtual_scrolling = enabled;
        self.item_height = item_height;
        self
    }

    /// Set the layout style for this container
    pub fn with_layout_style(mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.layout_style = layout_style.into();
        self
    }

    /// Get the current scroll offset
    pub fn scroll_offset(&self) -> Vector2<f32> {
        *self.scroll_offset.get()
    }

    /// Set the scroll offset
    pub fn set_scroll_offset(&self, offset: Vector2<f32>) {
        let clamped_offset = self.clamp_scroll_offset(offset);
        self.scroll_offset.set(clamped_offset);
    }

    /// Scroll to a specific position
    pub fn scroll_to(&self, x: f32, y: f32) {
        self.set_scroll_offset(Vector2::new(x, y));
    }

    /// Scroll by a delta amount
    pub fn scroll_by(&self, dx: f32, dy: f32) {
        let current = self.scroll_offset();
        let mut new_x = current.x + dx;
        let mut new_y = current.y + dy;
        
        // Respect scroll direction settings
        match self.scroll_direction {
            ScrollDirection::Vertical => new_x = current.x, // Don't scroll horizontally
            ScrollDirection::Horizontal => new_y = current.y, // Don't scroll vertically
            ScrollDirection::None => {
                new_x = current.x;
                new_y = current.y;
            },
            ScrollDirection::Both => {
                // Allow both directions
            },
        }
        
        self.set_scroll_offset(Vector2::new(new_x, new_y));
    }

    /// Scroll to ensure a rectangle is visible
    pub fn scroll_to_rect(&self, rect: Rect) {
        let current_offset = self.scroll_offset();
        let mut new_offset = current_offset;

        // Horizontal scrolling (only if direction allows it)
        if self.scroll_direction == ScrollDirection::Horizontal || self.scroll_direction == ScrollDirection::Both {
            if rect.x0 < current_offset.x as f64 {
                new_offset.x = rect.x0 as f32;
            } else if rect.x1 > (current_offset.x + self.viewport_size.x) as f64 {
                new_offset.x = (rect.x1 - self.viewport_size.x as f64) as f32;
            }
        }

        // Vertical scrolling (only if direction allows it)
        if self.scroll_direction == ScrollDirection::Vertical || self.scroll_direction == ScrollDirection::Both {
            if rect.y0 < current_offset.y as f64 {
                new_offset.y = rect.y0 as f32;
            } else if rect.y1 > (current_offset.y + self.viewport_size.y) as f64 {
                new_offset.y = (rect.y1 - self.viewport_size.y as f64) as f32;
            }
        }

        self.set_scroll_offset(new_offset);
    }


    fn clamp_scroll_offset(&self, offset: Vector2<f32>) -> Vector2<f32> {
        let max_x = (self.content_size.x - self.viewport_size.x).max(0.0);
        let max_y = (self.content_size.y - self.viewport_size.y).max(0.0);

        Vector2::new(offset.x.clamp(0.0, max_x), offset.y.clamp(0.0, max_y))
    }

    fn needs_vertical_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.y > self.viewport_size.y
                    && (self.scroll_direction == ScrollDirection::Vertical
                        || self.scroll_direction == ScrollDirection::Both)
            },
        }
    }

    fn needs_horizontal_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.x > self.viewport_size.x
                    && (self.scroll_direction == ScrollDirection::Horizontal
                        || self.scroll_direction == ScrollDirection::Both)
            },
        }
    }

    fn get_vertical_scrollbar_bounds(&self, layout: &LayoutNode) -> Rect {
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        let scrollbar_x = if self.vertical_scrollbar_position == VerticalScrollbarPosition::Left {
            container_bounds.x0
        } else {
            container_bounds.x1 - self.scrollbar_width as f64
        };
        let scrollbar_height = if self.needs_horizontal_scrollbar() {
            container_bounds.height() - self.scrollbar_width as f64
        } else {
            container_bounds.height()
        };

        Rect::new(
            scrollbar_x,
            container_bounds.y0,
            scrollbar_x + self.scrollbar_width as f64,
            container_bounds.y0 + scrollbar_height,
        )
    }

    fn get_horizontal_scrollbar_bounds(&self, layout: &LayoutNode) -> Rect {
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        let scrollbar_y = container_bounds.y1 - self.scrollbar_width as f64;
        let scrollbar_width = if self.needs_vertical_scrollbar() {
            container_bounds.width() - self.scrollbar_width as f64
        } else {
            container_bounds.width()
        };

        let scrollbar_x = if self.vertical_scrollbar_position == VerticalScrollbarPosition::Left
            && self.needs_vertical_scrollbar()
        {
            container_bounds.x0 + self.scrollbar_width as f64
        } else {
            container_bounds.x0
        };

        Rect::new(
            scrollbar_x,
            scrollbar_y,
            scrollbar_x + scrollbar_width,
            container_bounds.y1,
        )
    }

    fn get_vertical_up_button_bounds(&self, scrollbar_bounds: Rect) -> Rect {
        Rect::new(
            scrollbar_bounds.x0,
            scrollbar_bounds.y1 - (self.button_size * 2.0) as f64,
            scrollbar_bounds.x1,
            scrollbar_bounds.y1 - self.button_size as f64,
        )
    }

    fn get_vertical_down_button_bounds(&self, scrollbar_bounds: Rect) -> Rect {
        Rect::new(
            scrollbar_bounds.x0,
            scrollbar_bounds.y1 - self.button_size as f64,
            scrollbar_bounds.x1,
            scrollbar_bounds.y1,
        )
    }

    fn get_horizontal_left_button_bounds(&self, scrollbar_bounds: Rect) -> Rect {
        Rect::new(
            scrollbar_bounds.x0,
            scrollbar_bounds.y0,
            scrollbar_bounds.x0 + self.button_size as f64,
            scrollbar_bounds.y1,
        )
    }

    fn get_horizontal_right_button_bounds(&self, scrollbar_bounds: Rect) -> Rect {
        Rect::new(
            scrollbar_bounds.x1 - self.button_size as f64,
            scrollbar_bounds.y0,
            scrollbar_bounds.x1,
            scrollbar_bounds.y1,
        )
    }

    fn get_vertical_thumb_bounds(&self, scrollbar_bounds: Rect) -> Rect {
        if self.content_size.y <= self.viewport_size.y {
            return Rect::ZERO;
        }

        let button_space = if self.scrollbar_buttons == ScrollbarButtons::Always {
            (self.button_size * 2.0) as f64
        } else {
            0.0
        };

        let track_height = scrollbar_bounds.height() - button_space;
        if track_height <= 0.0 {
            return Rect::ZERO;
        }

        let thumb_height =
            (self.viewport_size.y / self.content_size.y * track_height as f32).max(20.0);
        let content_scrollable = self.content_size.y - self.viewport_size.y;
        // Guard against division by zero (shouldn't happen due to early return, but be safe)
        let scroll_ratio = if content_scrollable > 0.0 {
            self.scroll_offset().y / content_scrollable
        } else {
            0.0
        };
        let thumb_y =
            scrollbar_bounds.y0 + scroll_ratio as f64 * (track_height - thumb_height as f64);

        Rect::new(
            scrollbar_bounds.x0,
            thumb_y,
            scrollbar_bounds.x1,
            thumb_y + thumb_height as f64,
        )
    }

    fn get_horizontal_thumb_bounds(&self, scrollbar_bounds: Rect) -> Rect {
        if self.content_size.x <= self.viewport_size.x {
            return Rect::ZERO;
        }

        let button_space = if self.scrollbar_buttons == ScrollbarButtons::Always {
            (self.button_size * 2.0) as f64
        } else {
            0.0
        };

        let track_width = scrollbar_bounds.width() - button_space;
        if track_width <= 0.0 {
            return Rect::ZERO;
        }

        let thumb_width =
            (self.viewport_size.x / self.content_size.x * track_width as f32).max(20.0);
        let content_scrollable = self.content_size.x - self.viewport_size.x;
        // Guard against division by zero (shouldn't happen due to early return, but be safe)
        let scroll_ratio = if content_scrollable > 0.0 {
            self.scroll_offset().x / content_scrollable
        } else {
            0.0
        };
        // Track starts after left button, so add left button size to thumb position
        let left_button_size = if self.scrollbar_buttons == ScrollbarButtons::Always {
            self.button_size as f64
        } else {
            0.0
        };
        let thumb_x = scrollbar_bounds.x0
            + left_button_size
            + scroll_ratio as f64 * (track_width - thumb_width as f64);

        Rect::new(
            thumb_x,
            scrollbar_bounds.y0,
            thumb_x + thumb_width as f64,
            scrollbar_bounds.y1,
        )
    }

    fn render_scrollbar(
        &self,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        scrollbar_bounds: Rect,
        thumb_bounds: Rect,
        _is_vertical: bool,
        is_hovered: bool,
        is_pressed: bool,
    ) {
        // Draw scrollbar track
        let track_color = palette.color(ColorRole::ThreedShadow1);

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(track_color),
            None,
            &scrollbar_bounds.to_path(0.1),
        );

        // Draw scrollbar thumb
        let thumb_color = if is_pressed {
            palette.color(ColorRole::Selection)
        } else if is_hovered {
            palette.color(ColorRole::HoverHighlight)
        } else {
            palette.color(ColorRole::ThreedShadow2)
        };

        let thumb_rounded = RoundedRect::new(
            thumb_bounds.x0,
            thumb_bounds.y0,
            thumb_bounds.x1,
            thumb_bounds.y1,
            RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
        );

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(thumb_color),
            None,
            &thumb_rounded.to_path(0.1),
        );
    }

    fn render_scroll_button(
        &self,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        bounds: Rect,
        direction: ArrowDirection,
        is_hovered: bool,
        is_pressed: bool,
    ) {
        // Use scrollbar thumb colors for buttons
        let bg_color = if is_pressed {
            palette.color(ColorRole::Selection)
        } else if is_hovered {
            palette.color(ColorRole::HoverHighlight)
        } else {
            palette.color(ColorRole::ThreedShadow2)
        };
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &bounds.to_path(0.1),
        );

        // Use text color for arrow
        let arrow_color = palette.color(ColorRole::BaseText);
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) * 0.4;
        let mut path = BezPath::new();

        match direction {
            ArrowDirection::Up => {
                path.move_to(Point::new(center.x, center.y - size / 2.0));
                path.line_to(Point::new(center.x - size / 2.0, center.y + size / 2.0));
                path.line_to(Point::new(center.x + size / 2.0, center.y + size / 2.0));
                path.close_path();
            },
            ArrowDirection::Down => {
                path.move_to(Point::new(center.x, center.y + size / 2.0));
                path.line_to(Point::new(center.x - size / 2.0, center.y - size / 2.0));
                path.line_to(Point::new(center.x + size / 2.0, center.y - size / 2.0));
                path.close_path();
            },
            ArrowDirection::Left => {
                path.move_to(Point::new(center.x - size / 2.0, center.y));
                path.line_to(Point::new(center.x + size / 2.0, center.y - size / 2.0));
                path.line_to(Point::new(center.x + size / 2.0, center.y + size / 2.0));
                path.close_path();
            },
            ArrowDirection::Right => {
                path.move_to(Point::new(center.x + size / 2.0, center.y));
                path.line_to(Point::new(center.x - size / 2.0, center.y - size / 2.0));
                path.line_to(Point::new(center.x - size / 2.0, center.y + size / 2.0));
                path.close_path();
            },
        }

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(arrow_color),
            None,
            &path.to_path(0.1),
        );
    }

    fn update_visible_range(&mut self) {
        if !self.virtual_scrolling {
            return;
        }

        let scroll_y = self.scroll_offset().y;
        let viewport_height = self.viewport_size.y;

        let start_index = (scroll_y / self.item_height).floor() as usize;
        let visible_count = (viewport_height / self.item_height).ceil() as usize + 1; // +1 for partial items
        let end_index = start_index + visible_count;

        self.visible_range = (start_index, end_index);
    }

    /// Update child widget and calculate content size
    ///
    /// This method updates the child widget and measures its content size.
    /// If the child widget implements the `measure()` method, it can be used
    /// for more accurate initial sizing, especially when viewport size is unknown.
    async fn update_child_and_content_size(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        let mut update = Update::empty();

        if let Some(child) = &mut self.child {
            let child_layout = if !layout.children.is_empty() {
                &layout.children[0]
            } else {
                layout
            };

            let mut scrolled_layout = child_layout.clone();
            scrolled_layout.layout.location.x -= self.scroll_offset.get().x;
            scrolled_layout.layout.location.y -= self.scroll_offset.get().y;

            update |= child.update(&scrolled_layout, context, info).await;

            // Use layout size for content size
            // Note: If child implements measure(), it can provide more accurate
            // intrinsic sizing, but we rely on Taffy's computed layout here
            // for consistency with the layout system
            self.content_size = Vector2::new(
                child_layout.layout.size.width,
                child_layout.layout.size.height,
            );
        }

        update
    }

    /// Calculate viewport size and update scrollbar visibility state
    fn update_viewport_and_scrollbar_state(&mut self, layout: &LayoutNode) -> Update {
        let temp_viewport = Vector2::new(
            layout.layout.size.width,
            layout.layout.size.height,
        );
        
        let needs_vert = match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.y > temp_viewport.y
                    && (self.scroll_direction == ScrollDirection::Vertical
                        || self.scroll_direction == ScrollDirection::Both)
            },
        };
        let needs_horz = match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.x > temp_viewport.x
                    && (self.scroll_direction == ScrollDirection::Horizontal
                        || self.scroll_direction == ScrollDirection::Both)
            },
        };
        
        self.viewport_size = Vector2::new(
            layout.layout.size.width
                - if needs_vert {
                    self.scrollbar_width
                } else {
                    0.0
                },
            layout.layout.size.height
                - if needs_horz {
                    self.scrollbar_width
                } else {
                    0.0
                },
        );

        let scrollbar_state_changed = self.visibility_state.prev_needs_vertical_scrollbar != needs_vert
            || self.visibility_state.prev_needs_horizontal_scrollbar != needs_horz;

        self.visibility_state.prev_needs_vertical_scrollbar = needs_vert;
        self.visibility_state.prev_needs_horizontal_scrollbar = needs_horz;

        if scrollbar_state_changed {
            Update::LAYOUT
        } else {
            Update::empty()
        }
    }

    /// Update mouse hover states for scrollbars and buttons
    fn update_mouse_interactions(&mut self, layout: &LayoutNode) {
        if self.needs_vertical_scrollbar() {
            let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
            let thumb_bounds = self.get_vertical_thumb_bounds(scrollbar_bounds);

            self.scrollbar_state.vertical_scrollbar_hovered =
                thumb_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64));
        }

        if self.needs_horizontal_scrollbar() {
            let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
            let thumb_bounds = self.get_horizontal_thumb_bounds(scrollbar_bounds);

            self.scrollbar_state.horizontal_scrollbar_hovered =
                thumb_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64));
        }

        if self.scrollbar_buttons == ScrollbarButtons::Always {
            let mouse_point = Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64);
            if self.needs_vertical_scrollbar() {
                let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
                self.button_state.up_button_hovered = self
                    .get_vertical_up_button_bounds(scrollbar_bounds)
                    .contains(mouse_point);
                self.button_state.down_button_hovered = self
                    .get_vertical_down_button_bounds(scrollbar_bounds)
                    .contains(mouse_point);
            }
            if self.needs_horizontal_scrollbar() {
                let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
                self.button_state.left_button_hovered = self
                    .get_horizontal_left_button_bounds(scrollbar_bounds)
                    .contains(mouse_point);
                self.button_state.right_button_hovered = self
                    .get_horizontal_right_button_bounds(scrollbar_bounds)
                    .contains(mouse_point);
            }
        }
    }

    /// Handle mouse wheel scrolling with natural scrolling support
    fn handle_mouse_wheel_scrolling(&mut self, scroll_delta: MouseScrollDelta, context: &AppContext) -> Update {
        let base_scroll_speed = 40.0;
        let natural_scrolling = context.settings.get().mouse.natural_scrolling.unwrap_or(false);
        let direction_multiplier = if natural_scrolling { -1.0 } else { 1.0 };

        let (dx, dy) = match scroll_delta {
            MouseScrollDelta::LineDelta(x, y) => {
                let vertical_scale = if self.viewport_size.y > 0.0 {
                    (self.content_size.y / self.viewport_size.y)
                        .min(5.0)
                        .max(1.0)
                } else {
                    1.0
                };
                let horizontal_scale = if self.viewport_size.x > 0.0 {
                    (self.content_size.x / self.viewport_size.x)
                        .min(5.0)
                        .max(1.0)
                } else {
                    1.0
                };

                let raw_dx = x * base_scroll_speed * horizontal_scale * direction_multiplier;
                let raw_dy = y * base_scroll_speed * vertical_scale * direction_multiplier;

                let max_step_y = (self.viewport_size.y / 3.0).min(200.0).max(50.0);
                let max_step_x = (self.viewport_size.x / 3.0).min(200.0).max(50.0);

                (
                    raw_dx.clamp(-max_step_x, max_step_x),
                    raw_dy.clamp(-max_step_y, max_step_y),
                )
            },
            MouseScrollDelta::PixelDelta(pos) => {
                (
                    pos.x as f32 * direction_multiplier,
                    pos.y as f32 * direction_multiplier,
                )
            },
        };

        let old_offset = self.scroll_offset();
        self.scroll_by(dx, dy);
        let new_offset = self.scroll_offset();

        if old_offset != new_offset {
            Update::DRAW
        } else {
            Update::empty()
        }
    }

    /// Handle mouse button press/release events for scrollbar interaction
    fn handle_mouse_button_events(&mut self, layout: &LayoutNode, button: MouseButton, state: ElementState) {
        if button != MouseButton::Left {
            return;
        }

        match state {
            ElementState::Pressed => {
                if self.button_state.up_button_hovered {
                    self.button_state.button_held = Some(ArrowDirection::Up);
                } else if self.button_state.down_button_hovered {
                    self.button_state.button_held = Some(ArrowDirection::Down);
                } else if self.button_state.left_button_hovered {
                    self.button_state.button_held = Some(ArrowDirection::Left);
                } else if self.button_state.right_button_hovered {
                    self.button_state.button_held = Some(ArrowDirection::Right);
                }

                if self.needs_vertical_scrollbar() {
                    let thumb_bounds = self.get_vertical_thumb_bounds(
                        self.get_vertical_scrollbar_bounds(layout),
                    );
                    if thumb_bounds.contains(Point::new(
                        self.mouse_pos.x as f64,
                        self.mouse_pos.y as f64,
                    )) {
                        self.scrollbar_state.dragging_vertical = true;
                        self.scrollbar_state.drag_start_pos = self.mouse_pos;
                        self.scrollbar_state.drag_start_offset = self.scroll_offset();
                    }
                }

                if self.needs_horizontal_scrollbar() {
                    let thumb_bounds = self.get_horizontal_thumb_bounds(
                        self.get_horizontal_scrollbar_bounds(layout),
                    );
                    if thumb_bounds.contains(Point::new(
                        self.mouse_pos.x as f64,
                        self.mouse_pos.y as f64,
                    )) {
                        self.scrollbar_state.dragging_horizontal = true;
                        self.scrollbar_state.drag_start_pos = self.mouse_pos;
                        self.scrollbar_state.drag_start_offset = self.scroll_offset();
                    }
                }
            },
            ElementState::Released => {
                self.button_state.button_held = None;
                self.scrollbar_state.dragging_vertical = false;
                self.scrollbar_state.dragging_horizontal = false;
            },
        }
    }

    /// Handle scrollbar thumb dragging
    fn handle_scrollbar_dragging(&mut self, layout: &LayoutNode) -> Update {
        if !self.scrollbar_state.dragging_vertical && !self.scrollbar_state.dragging_horizontal {
            return Update::empty();
        }

        let mouse_delta = self.mouse_pos - self.scrollbar_state.drag_start_pos;
        let mut new_offset = self.scrollbar_state.drag_start_offset;

        if self.scrollbar_state.dragging_vertical && self.content_size.y > self.viewport_size.y {
            let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
            let thumb_bounds = self.get_vertical_thumb_bounds(scrollbar_bounds);

            let available_scrollbar_space =
                scrollbar_bounds.height() as f32 - thumb_bounds.height() as f32;
            let available_content_space = self.content_size.y - self.viewport_size.y;

            if available_scrollbar_space > 0.0 {
                let scroll_ratio = mouse_delta.y / available_scrollbar_space;
                new_offset.y =
                    self.scrollbar_state.drag_start_offset.y + (scroll_ratio * available_content_space);
                new_offset.y = new_offset.y.max(0.0).min(available_content_space);
            }
        }

        if self.scrollbar_state.dragging_horizontal && self.content_size.x > self.viewport_size.x {
            let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
            let thumb_bounds = self.get_horizontal_thumb_bounds(scrollbar_bounds);

            let available_scrollbar_space =
                scrollbar_bounds.width() as f32 - thumb_bounds.width() as f32;
            let available_content_space = self.content_size.x - self.viewport_size.x;

            if available_scrollbar_space > 0.0 {
                let scroll_ratio = mouse_delta.x / available_scrollbar_space;
                new_offset.x =
                    self.scrollbar_state.drag_start_offset.x + (scroll_ratio * available_content_space);
                new_offset.x = new_offset.x.max(0.0).min(available_content_space);
            }
        }

        let old_offset = self.scroll_offset();
        self.set_scroll_offset(new_offset);

        if old_offset != self.scroll_offset() {
            Update::DRAW
        } else {
            Update::empty()
        }
    }

    /// Update previous state fields for change detection
    fn update_previous_states(&mut self) {
        self.scrollbar_state.prev_dragging_vertical = self.scrollbar_state.dragging_vertical;
        self.scrollbar_state.prev_dragging_horizontal = self.scrollbar_state.dragging_horizontal;
        self.button_state.prev_button_held = self.button_state.button_held;
        self.button_state.prev_up_button_hovered = self.button_state.up_button_hovered;
        self.button_state.prev_down_button_hovered = self.button_state.down_button_hovered;
        self.button_state.prev_left_button_hovered = self.button_state.left_button_hovered;
        self.button_state.prev_right_button_hovered = self.button_state.right_button_hovered;
    }

    /// Calculate viewport size for rendering (accounts for scrollbars)
    fn calculate_viewport_size_for_render(&mut self, layout: &LayoutNode) {
        let temp_viewport = Vector2::new(
            layout.layout.size.width,
            layout.layout.size.height,
        );
        
        let needs_vert = match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.y > temp_viewport.y
                    && (self.scroll_direction == ScrollDirection::Vertical
                        || self.scroll_direction == ScrollDirection::Both)
            },
        };
        let needs_horz = match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.x > temp_viewport.x
                    && (self.scroll_direction == ScrollDirection::Horizontal
                        || self.scroll_direction == ScrollDirection::Both)
            },
        };
        
        self.viewport_size = Vector2::new(
            layout.layout.size.width
                - if needs_vert {
                    self.scrollbar_width
                } else {
                    0.0
                },
            layout.layout.size.height
                - if needs_horz {
                    self.scrollbar_width
                } else {
                    0.0
                },
        );
    }

    /// Render container border
    fn render_container_border(
        &self,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        container_bounds: Rect,
    ) {
        let border_color = palette.color(ColorRole::ThreedShadow1);
        let stroke = Stroke::new(1.0);
        graphics.stroke(
            &stroke,
            Affine::IDENTITY,
            &Brush::Solid(border_color),
            None,
            &container_bounds.to_path(0.1),
        );
    }

    /// Render child content with clipping and scrolling
    fn render_child_content(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        if let Some(child) = &mut self.child {
            let base_layout = if !layout.children.is_empty() {
                &layout.children[0]
            } else {
                layout
            };

            let content_rect = Rect::new(
                (layout.layout.location.x + layout.layout.padding.left) as f64,
                (layout.layout.location.y + layout.layout.padding.top) as f64,
                (layout.layout.location.x + layout.layout.size.width - layout.layout.padding.right)
                    as f64,
                (layout.layout.location.y + layout.layout.size.height - layout.layout.padding.bottom)
                    as f64,
            );

            #[allow(deprecated)]
            graphics.push_layer(Mix::Clip, 1.0, Affine::IDENTITY, &content_rect.to_path(0.1));

            let mut scrolled_layout = base_layout.clone();
            scrolled_layout.layout.location.x -= self.scroll_offset.get().x;
            scrolled_layout.layout.location.y -= self.scroll_offset.get().y;

            child.render(graphics, &scrolled_layout, info, context);

            graphics.pop_layer();
        }
    }

    /// Render vertical scrollbar with buttons if enabled
    fn render_vertical_scrollbar_with_buttons(
        &self,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        layout: &LayoutNode,
    ) {
        if !self.needs_vertical_scrollbar() {
            return;
        }

        let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
        let thumb_bounds = self.get_vertical_thumb_bounds(scrollbar_bounds);
        self.render_scrollbar(
            graphics,
            palette,
            scrollbar_bounds,
            thumb_bounds,
            true,
            self.scrollbar_state.vertical_scrollbar_hovered,
            self.scrollbar_state.dragging_vertical,
        );

        if self.scrollbar_buttons == ScrollbarButtons::Always {
            let up_button_bounds = self.get_vertical_up_button_bounds(scrollbar_bounds);
            let down_button_bounds = self.get_vertical_down_button_bounds(scrollbar_bounds);
            self.render_scroll_button(
                graphics,
                palette,
                up_button_bounds,
                ArrowDirection::Up,
                self.button_state.up_button_hovered,
                self.button_state.button_held == Some(ArrowDirection::Up),
            );
            self.render_scroll_button(
                graphics,
                palette,
                down_button_bounds,
                ArrowDirection::Down,
                self.button_state.down_button_hovered,
                self.button_state.button_held == Some(ArrowDirection::Down),
            );
        }
    }

    /// Render horizontal scrollbar with buttons if enabled
    fn render_horizontal_scrollbar_with_buttons(
        &self,
        graphics: &mut dyn Graphics,
        palette: &Palette,
        layout: &LayoutNode,
    ) {
        if !self.needs_horizontal_scrollbar() {
            return;
        }

        let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
        let thumb_bounds = self.get_horizontal_thumb_bounds(scrollbar_bounds);
        self.render_scrollbar(
            graphics,
            palette,
            scrollbar_bounds,
            thumb_bounds,
            false,
            self.scrollbar_state.horizontal_scrollbar_hovered,
            self.scrollbar_state.dragging_horizontal,
        );

        if self.scrollbar_buttons == ScrollbarButtons::Always {
            let left_button_bounds = self.get_horizontal_left_button_bounds(scrollbar_bounds);
            let right_button_bounds = self.get_horizontal_right_button_bounds(scrollbar_bounds);
            self.render_scroll_button(
                graphics,
                palette,
                left_button_bounds,
                ArrowDirection::Left,
                self.button_state.left_button_hovered,
                self.button_state.button_held == Some(ArrowDirection::Left),
            );
            self.render_scroll_button(
                graphics,
                palette,
                right_button_bounds,
                ArrowDirection::Right,
                self.button_state.right_button_hovered,
                self.button_state.button_held == Some(ArrowDirection::Right),
            );
        }
    }

    /// Calculate container size for a given direction, accounting for previous scrollbar state
    fn calculate_container_size_for_direction(&self, is_vertical: bool) -> f32 {
        if is_vertical {
            if self.visibility_state.prev_needs_vertical_scrollbar {
                self.viewport_size.y + self.scrollbar_width
            } else if self.viewport_size.y > 0.0 {
                self.viewport_size.y
            } else {
                // viewport_size is 0.0 (initial state) - use heuristic
                if self.content_size.y > 50.0 {
                    self.content_size.y - 1.0
                } else {
                    self.content_size.y + 1.0
                }
            }
        } else {
            if self.visibility_state.prev_needs_horizontal_scrollbar {
                self.viewport_size.x + self.scrollbar_width
            } else if self.viewport_size.x > 0.0 {
                self.viewport_size.x
            } else {
                // viewport_size is 0.0 (initial state) - use heuristic
                if self.content_size.x > 50.0 {
                    self.content_size.x - 1.0
                } else {
                    self.content_size.x + 1.0
                }
            }
        }
    }

    /// Determine if scrollbar should be shown for a given direction
    fn should_show_scrollbar(&self, is_vertical: bool, container_size: f32) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                let content_size = if is_vertical {
                    self.content_size.y
                } else {
                    self.content_size.x
                };
                let direction_allows = if is_vertical {
                    self.scroll_direction == ScrollDirection::Vertical
                        || self.scroll_direction == ScrollDirection::Both
                } else {
                    self.scroll_direction == ScrollDirection::Horizontal
                        || self.scroll_direction == ScrollDirection::Both
                };
                content_size > container_size && direction_allows
            },
        }
    }

    /// Calculate scrollbar needs for layout style calculation
    fn calculate_scrollbar_needs_for_layout(&self) -> (bool, bool) {
        // Use overflow detection if content size is available
        let use_overflow_detection = self.content_size.x > 0.0 || self.content_size.y > 0.0;
        
        let needs_vert = if self.visibility_state.prev_needs_vertical_scrollbar {
            true
        } else if self.content_size.y == 0.0 && self.content_size.x == 0.0 {
            matches!(self.scrollbar_visibility, ScrollbarVisibility::Always)
        } else {
            let container_height = self.calculate_container_size_for_direction(true);
            
            // Use overflow detection for more accurate scrollbar visibility
            if use_overflow_detection {
                let container_size = Vector2::new(self.viewport_size.x, container_height);
                let overflow = OverflowDetector::exceeds_bounds(container_size, self.content_size);
                overflow || self.should_show_scrollbar(true, container_height)
            } else {
                self.should_show_scrollbar(true, container_height)
            }
        };

        let needs_horz = if self.visibility_state.prev_needs_horizontal_scrollbar {
            true
        } else if self.content_size.y == 0.0 && self.content_size.x == 0.0 {
            matches!(self.scrollbar_visibility, ScrollbarVisibility::Always)
        } else {
            let container_width = self.calculate_container_size_for_direction(false);
            
            // Use overflow detection for more accurate scrollbar visibility
            if use_overflow_detection {
                let container_size = Vector2::new(container_width, self.viewport_size.y);
                let overflow = OverflowDetector::exceeds_bounds(container_size, self.content_size);
                overflow || self.should_show_scrollbar(false, container_width)
            } else {
                self.should_show_scrollbar(false, container_width)
            }
        };

        (needs_vert, needs_horz)
    }

    /// Apply scrollbar padding to layout style
    fn apply_scrollbar_padding(&self, style: &mut LayoutStyle, needs_vert: bool, needs_horz: bool) {
        if needs_vert {
            match self.vertical_scrollbar_position {
                VerticalScrollbarPosition::Left => {
                    style.padding.left = LengthPercentage::length(self.scrollbar_width)
                },
                VerticalScrollbarPosition::Right => {
                    style.padding.right = LengthPercentage::length(self.scrollbar_width)
                },
            }
        }
        if needs_horz {
            style.padding.bottom = LengthPercentage::length(self.scrollbar_width);
        }
    }
}

impl Default for ScrollContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for ScrollContainer {

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) -> () {
        let palette = context.palette();
        
        // Calculate viewport size accounting for scrollbars
        self.calculate_viewport_size_for_render(layout);

        // Draw container border
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );
        self.render_container_border(graphics, palette, container_bounds);

        // Render child content with clipping and scrolling
        self.render_child_content(graphics, layout, info, context.clone());

        // Update visible range for virtual scrolling
        self.update_visible_range();

        // Render scrollbars
        self.render_vertical_scrollbar_with_buttons(graphics, palette, layout);
        self.render_horizontal_scrollbar_with_buttons(graphics, palette, layout);
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Update child and content size
        update |= self.update_child_and_content_size(layout, context.clone(), info).await;

        // Update viewport size and scrollbar visibility state
        update |= self.update_viewport_and_scrollbar_state(layout);

        // Update mouse position
        if let Some(cursor_pos) = info.cursor_pos {
            self.mouse_pos = Vector2::new(cursor_pos.x as f32, cursor_pos.y as f32);
        }

        // Update mouse interactions (hover states)
        self.update_mouse_interactions(layout);

        // Handle mouse wheel scrolling
        if let Some(scroll_delta) = info.mouse_scroll_delta {
            update |= self.handle_mouse_wheel_scrolling(scroll_delta, &context);
        }

        // Handle mouse button events
        for (_, button, state) in &info.buttons {
            self.handle_mouse_button_events(layout, *button, *state);
        }

        // Handle continuous scrolling when a button is held
        if let Some(direction) = self.button_state.button_held {
            let old_offset = self.scroll_offset();
            let scroll_step = 5.0;
            match direction {
                ArrowDirection::Up => self.scroll_by(0.0, -scroll_step),
                ArrowDirection::Down => self.scroll_by(0.0, scroll_step),
                ArrowDirection::Left => self.scroll_by(-scroll_step, 0.0),
                ArrowDirection::Right => self.scroll_by(scroll_step, 0.0),
            }
            if self.scroll_offset() != old_offset {
                update |= Update::DRAW;
            }
        }

        // Handle scrollbar dragging
        update |= self.handle_scrollbar_dragging(layout);

        // Force redraw if hover or pressed states change
        if self.scrollbar_state.dragging_vertical != self.scrollbar_state.prev_dragging_vertical
            || self.scrollbar_state.dragging_horizontal != self.scrollbar_state.prev_dragging_horizontal
            || self.button_state.button_held != self.button_state.prev_button_held
            || self.button_state.up_button_hovered != self.button_state.prev_up_button_hovered
            || self.button_state.down_button_hovered != self.button_state.prev_down_button_hovered
            || self.button_state.left_button_hovered != self.button_state.prev_left_button_hovered
            || self.button_state.right_button_hovered != self.button_state.prev_right_button_hovered
        {
            update |= Update::DRAW;
        }

        // Update previous states
        self.update_previous_states();

        update
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        let mut style = self.layout_style.get().clone();

        // Calculate scrollbar needs (needed for both measure and layout phases)
        let (needs_vert, needs_horz) = self.calculate_scrollbar_needs_for_layout();

        // In measure phase, use loose constraints to get intrinsic sizes
        // In layout phase, calculate scrollbar needs and apply padding
        if !context.phase.is_measure() {
            // During layout phase, apply scrollbar padding
            self.apply_scrollbar_padding(&mut style, needs_vert, needs_horz);
        }

        StyleNode {
            style,
            children: if let Some(child) = &self.child {
                // Create enhanced context with viewport info for layout-level culling
                let scroll_offset = *self.scroll_offset.get(); // Clone the value
                let mut child_context = context.clone();
                
                // Calculate viewport bounds relative to scroll container
                // The viewport is the visible area after accounting for scrollbars
                let viewport_width = if needs_horz {
                    self.viewport_size.x - self.scrollbar_width
                } else {
                    self.viewport_size.x
                };
                let viewport_height = if needs_vert {
                    self.viewport_size.y - self.scrollbar_width
                } else {
                    self.viewport_size.y
                };
                
                // Viewport bounds are relative to the scroll container's content area
                // Adjust for scroll offset (content is offset by negative scroll)
                let viewport_bounds = ViewportBounds::new(
                    -scroll_offset.x, // Content offset by negative scroll
                    -scroll_offset.y,
                    viewport_width,
                    viewport_height,
                );
                
                child_context = child_context
                    .with_viewport_bounds(viewport_bounds)
                    .with_scroll_offset(scroll_offset);
                
                vec![child.layout_style(&child_context)]
            } else {
                vec![]
            },
            measure_func: None,
        }
    }
}

impl WidgetLayoutExt for ScrollContainer {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}
