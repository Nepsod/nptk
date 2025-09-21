use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, Dimension, StyleNode, LengthPercentageAuto, LengthPercentage};
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke, Point, BezPath};
use nptk_core::vg::peniko::{Fill, Color, Mix};
use nptk_core::vg::Scene;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton, MouseScrollDelta};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;

/// Vertical scrollbar position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalScrollbarPosition {
    Left,
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
    Always,
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
    last_scroll_offset: Vector2<f32>,
    content_size: Vector2<f32>,
    viewport_size: Vector2<f32>,
    
    // Scrollbar interaction state
    dragging_vertical: bool,
    dragging_horizontal: bool,
    drag_start_pos: Vector2<f32>,
    drag_start_offset: Vector2<f32>,
    
    // Mouse state
    mouse_pos: Vector2<f32>,
    vertical_scrollbar_hovered: bool,
    horizontal_scrollbar_hovered: bool,
    up_button_hovered: bool,
    down_button_hovered: bool,
    left_button_hovered: bool,
    right_button_hovered: bool,
    button_held: Option<ArrowDirection>,
    
    // Previous states for change detection
    prev_dragging_vertical: bool,
    prev_dragging_horizontal: bool,
    prev_button_held: Option<ArrowDirection>,
    prev_up_button_hovered: bool,
    prev_down_button_hovered: bool,
    prev_left_button_hovered: bool,
    prev_right_button_hovered: bool,
    
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
                size: Vector2::new(
                    Dimension::percent(1.0),
                    Dimension::percent(1.0),
                ),
                overflow: (nptk_core::layout::Overflow::Hidden, nptk_core::layout::Overflow::Hidden),
                ..Default::default()
            }.into(),
            scroll_direction: ScrollDirection::Both,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            scrollbar_width: 12.0,
            vertical_scrollbar_position: VerticalScrollbarPosition::default(),
            scrollbar_buttons: ScrollbarButtons::default(),
            button_size: 16.0,
            scroll_offset: StateSignal::new(Vector2::new(0.0, 0.0)),
            last_scroll_offset: Vector2::new(0.0, 0.0),
            content_size: Vector2::new(0.0, 0.0),
            viewport_size: Vector2::new(0.0, 0.0),
            dragging_vertical: false,
            dragging_horizontal: false,
            drag_start_pos: Vector2::new(0.0, 0.0),
            drag_start_offset: Vector2::new(0.0, 0.0),
            mouse_pos: Vector2::new(0.0, 0.0),
            vertical_scrollbar_hovered: false,
            horizontal_scrollbar_hovered: false,
            up_button_hovered: false,
            down_button_hovered: false,
            left_button_hovered: false,
            right_button_hovered: false,
            button_held: None,
            prev_dragging_vertical: false,
            prev_dragging_horizontal: false,
            prev_button_held: None,
            prev_up_button_hovered: false,
            prev_down_button_hovered: false,
            prev_left_button_hovered: false,
            prev_right_button_hovered: false,
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
        self.set_scroll_offset(Vector2::new(current.x + dx, current.y + dy));
    }

    /// Scroll to ensure a rectangle is visible
    pub fn scroll_to_rect(&self, rect: Rect) {
        let current_offset = self.scroll_offset();
        let mut new_offset = current_offset;

        // Horizontal scrolling
        if rect.x0 < current_offset.x as f64 {
            new_offset.x = rect.x0 as f32;
        } else if rect.x1 > (current_offset.x + self.viewport_size.x) as f64 {
            new_offset.x = (rect.x1 - self.viewport_size.x as f64) as f32;
        }

        // Vertical scrolling
        if rect.y0 < current_offset.y as f64 {
            new_offset.y = rect.y0 as f32;
        } else if rect.y1 > (current_offset.y + self.viewport_size.y) as f64 {
            new_offset.y = (rect.y1 - self.viewport_size.y as f64) as f32;
        }

        self.set_scroll_offset(new_offset);
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "ScrollContainer")
    }

    fn clamp_scroll_offset(&self, offset: Vector2<f32>) -> Vector2<f32> {
        let max_x = (self.content_size.x - self.viewport_size.x).max(0.0);
        let max_y = (self.content_size.y - self.viewport_size.y).max(0.0);
        
        Vector2::new(
            offset.x.clamp(0.0, max_x),
            offset.y.clamp(0.0, max_y),
        )
    }

    fn needs_vertical_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.y > self.viewport_size.y &&
                (self.scroll_direction == ScrollDirection::Vertical || self.scroll_direction == ScrollDirection::Both)
            }
        }
    }

    fn needs_horizontal_scrollbar(&self) -> bool {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::Never => false,
            ScrollbarVisibility::Auto => {
                self.content_size.x > self.viewport_size.x &&
                (self.scroll_direction == ScrollDirection::Horizontal || self.scroll_direction == ScrollDirection::Both)
            }
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
        
        let scrollbar_x = if self.vertical_scrollbar_position == VerticalScrollbarPosition::Left && self.needs_vertical_scrollbar() {
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
            scrollbar_bounds.x0 + self.button_size as f64,
            scrollbar_bounds.y0,
            scrollbar_bounds.x0 + (self.button_size * 2.0) as f64,
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
        if track_height <= 0.0 { return Rect::ZERO; }

        let thumb_height = (self.viewport_size.y / self.content_size.y * track_height as f32).max(20.0);
        let scroll_ratio = self.scroll_offset().y / (self.content_size.y - self.viewport_size.y);
        let thumb_y = scrollbar_bounds.y0 + scroll_ratio as f64 * (track_height - thumb_height as f64);

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
        if track_width <= 0.0 { return Rect::ZERO; }

        let thumb_width = (self.viewport_size.x / self.content_size.x * track_width as f32).max(20.0);
        let scroll_ratio = self.scroll_offset().x / (self.content_size.x - self.viewport_size.x);
        let thumb_x = scrollbar_bounds.x0 + button_space + scroll_ratio as f64 * (track_width - thumb_width as f64);

        Rect::new(
            thumb_x,
            scrollbar_bounds.y0,
            thumb_x + thumb_width as f64,
            scrollbar_bounds.y1,
        )
    }

    fn render_scrollbar(&self, scene: &mut Scene, theme: Option<&nptk_theme::style::Style>, scrollbar_bounds: Rect, thumb_bounds: Rect, _is_vertical: bool, is_hovered: bool, is_pressed: bool) {
        // Draw scrollbar track
        let track_color = if let Some(style) = theme {
            style.get_color("color_scrollbar").unwrap_or(Color::from_rgb8(230, 230, 230))
        } else {
            Color::from_rgb8(230, 230, 230)
        };

        scene.fill(Fill::NonZero, Affine::IDENTITY, track_color, None, &scrollbar_bounds);

        // Draw scrollbar thumb
        let thumb_color = if let Some(style) = theme {
            if is_pressed {
                style.get_color("color_scrollbar_thumb_active").unwrap_or(Color::from_rgb8(120, 120, 120))
            } else if is_hovered {
                style.get_color("color_scrollbar_thumb_hover").unwrap_or(Color::from_rgb8(150, 150, 150))
            } else {
                style.get_color("color_scrollbar_thumb").unwrap_or(Color::from_rgb8(180, 180, 180))
            }
        } else if is_pressed {
            Color::from_rgb8(120, 120, 120)
        } else if is_hovered {
            Color::from_rgb8(150, 150, 150)
        } else {
            Color::from_rgb8(180, 180, 180)
        };

        let thumb_rounded = RoundedRect::new(
            thumb_bounds.x0,
            thumb_bounds.y0,
            thumb_bounds.x1,
            thumb_bounds.y1,
            RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
        );

        scene.fill(Fill::NonZero, Affine::IDENTITY, thumb_color, None, &thumb_rounded);
    }

    fn render_scroll_button(&self, scene: &mut Scene, theme: Option<&nptk_theme::style::Style>, bounds: Rect, direction: ArrowDirection, is_hovered: bool, is_pressed: bool) {
        let bg_color = if let Some(style) = theme {
            if is_pressed {
                style.get_color("color_scrollbar_thumb_active").unwrap_or(Color::from_rgb8(120, 120, 120))
            } else if is_hovered {
                style.get_color("color_scrollbar_thumb_hover").unwrap_or(Color::from_rgb8(150, 150, 150))
            } else {
                style.get_color("color_scrollbar_thumb").unwrap_or(Color::from_rgb8(180, 180, 180))
            }
        } else if is_pressed {
            Color::from_rgb8(120, 120, 120)
        } else if is_hovered {
            Color::from_rgb8(150, 150, 150)
        } else {
            Color::from_rgb8(180, 180, 180)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &bounds);
    
        let arrow_color = Color::BLACK;
        let center = bounds.center();
        let size = bounds.width().min(bounds.height()) * 0.4;
        let mut path = BezPath::new();
    
        match direction {
            ArrowDirection::Up => {
                path.move_to(Point::new(center.x, center.y - size / 2.0));
                path.line_to(Point::new(center.x - size / 2.0, center.y + size / 2.0));
                path.line_to(Point::new(center.x + size / 2.0, center.y + size / 2.0));
                path.close_path();
            }
            ArrowDirection::Down => {
                path.move_to(Point::new(center.x, center.y + size / 2.0));
                path.line_to(Point::new(center.x - size / 2.0, center.y - size / 2.0));
                path.line_to(Point::new(center.x + size / 2.0, center.y - size / 2.0));
                path.close_path();
            }
            ArrowDirection::Left => {
                path.move_to(Point::new(center.x - size / 2.0, center.y));
                path.line_to(Point::new(center.x + size / 2.0, center.y - size / 2.0));
                path.line_to(Point::new(center.x + size / 2.0, center.y + size / 2.0));
                path.close_path();
            }
            ArrowDirection::Right => {
                path.move_to(Point::new(center.x + size / 2.0, center.y));
                path.line_to(Point::new(center.x - size / 2.0, center.y - size / 2.0));
                path.line_to(Point::new(center.x - size / 2.0, center.y + size / 2.0));
                path.close_path();
            }
        }
        
        scene.fill(Fill::NonZero, Affine::IDENTITY, arrow_color, None, &path);
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
}

impl Default for ScrollContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ScrollContainer {
    fn widget_id(&self) -> WidgetId {
        self.widget_id()
    }

    fn render(&mut self, scene: &mut Scene, theme: &mut dyn Theme, layout: &LayoutNode, _info: &AppInfo, context: AppContext) -> () {
        let widget_theme = theme.of(self.widget_id());
        let widget_theme_ref = widget_theme.as_ref();

        // Update viewport size
        self.viewport_size = Vector2::new(
            layout.layout.size.width - if self.needs_vertical_scrollbar() { self.scrollbar_width } else { 0.0 },
            layout.layout.size.height - if self.needs_horizontal_scrollbar() { self.scrollbar_width } else { 0.0 },
        );

        // Draw container background
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        let bg_color = if let Some(style) = widget_theme_ref {
            style.get_color("color_background").unwrap_or(Color::WHITE)
        } else {
            Color::WHITE
        };

        // Calculate content area (excluding scrollbars)
        let content_width = layout.layout.size.width - if self.needs_vertical_scrollbar() { self.scrollbar_width } else { 0.0 };
        let content_height = layout.layout.size.height - if self.needs_horizontal_scrollbar() { self.scrollbar_width } else { 0.0 };
        
        // Define content clipping rectangle
        let content_rect = Rect::new(
            (layout.layout.location.x + layout.layout.padding.left) as f64,
            (layout.layout.location.y + layout.layout.padding.top) as f64,
            (layout.layout.location.x + layout.layout.size.width - layout.layout.padding.right) as f64,
            (layout.layout.location.y + layout.layout.size.height - layout.layout.padding.bottom) as f64,
        );

        // Don't fill content area background to avoid overlapping scrolled content
        // scene.fill(Fill::NonZero, Affine::IDENTITY, bg_color, None, &content_rect);

        // Draw border
        let border_color = if let Some(style) = widget_theme_ref {
            style.get_color("color_border").unwrap_or(Color::from_rgb8(200, 200, 200))
        } else {
            Color::from_rgb8(200, 200, 200)
        };
        let stroke = Stroke::new(1.0);
        scene.stroke(&stroke, Affine::IDENTITY, border_color, None, &container_bounds);
        
        // Render child content using layout-based scrolling
        if let Some(child) = &mut self.child {
            // Use child layout if available
            let base_layout = if !layout.children.is_empty() {
                &layout.children[0]
            } else {
                layout
            };

            // Apply clipping to content area
            scene.push_layer(Mix::Clip, 1.0, Affine::IDENTITY, &content_rect);
            
            // Create a temporary, scrolled layout node to pass to the child.
            // This is much more performant than a full re-layout.
            let mut scrolled_layout = base_layout.clone();
            scrolled_layout.layout.location.x -= self.scroll_offset.get().x;
            scrolled_layout.layout.location.y -= self.scroll_offset.get().y;

            child.render(scene, theme, &scrolled_layout, _info, context);

            // Pop the clipping layer
            scene.pop_layer();
        }

        // Update visible range for virtual scrolling
        self.update_visible_range();

        // Draw scrollbars
        if self.needs_vertical_scrollbar() {
            let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
            let thumb_bounds = self.get_vertical_thumb_bounds(scrollbar_bounds);
            self.render_scrollbar(scene, widget_theme_ref, scrollbar_bounds, thumb_bounds, true, self.vertical_scrollbar_hovered, self.dragging_vertical);

            if self.scrollbar_buttons == ScrollbarButtons::Always {
                let up_button_bounds = self.get_vertical_up_button_bounds(scrollbar_bounds);
                let down_button_bounds = self.get_vertical_down_button_bounds(scrollbar_bounds);
                self.render_scroll_button(scene, widget_theme_ref, up_button_bounds, ArrowDirection::Up, self.up_button_hovered, self.button_held == Some(ArrowDirection::Up));
                self.render_scroll_button(scene, widget_theme_ref, down_button_bounds, ArrowDirection::Down, self.down_button_hovered, self.button_held == Some(ArrowDirection::Down));
            }
        }

        if self.needs_horizontal_scrollbar() {
            let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
            let thumb_bounds = self.get_horizontal_thumb_bounds(scrollbar_bounds);
            self.render_scrollbar(scene, widget_theme_ref, scrollbar_bounds, thumb_bounds, false, self.horizontal_scrollbar_hovered, self.dragging_horizontal);

            if self.scrollbar_buttons == ScrollbarButtons::Always {
                let left_button_bounds = self.get_horizontal_left_button_bounds(scrollbar_bounds);
                let right_button_bounds = self.get_horizontal_right_button_bounds(scrollbar_bounds);
                self.render_scroll_button(scene, widget_theme_ref, left_button_bounds, ArrowDirection::Left, self.left_button_hovered, self.button_held == Some(ArrowDirection::Left));
                self.render_scroll_button(scene, widget_theme_ref, right_button_bounds, ArrowDirection::Right, self.right_button_hovered, self.button_held == Some(ArrowDirection::Right));
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &AppInfo) -> Update {
        let mut update = Update::empty();

        // Update child
        if let Some(child) = &mut self.child {
            // Use the first child's layout if available, otherwise use the container's layout
            let child_layout = if !layout.children.is_empty() {
                &layout.children[0]
            } else {
                layout
            };
            
            update |= child.update(child_layout, context, info);

            // Update content size based on child's layout
            self.content_size = Vector2::new(
                child_layout.layout.size.width,
                child_layout.layout.size.height,
            );
        }

        // Update mouse position
        if let Some(cursor_pos) = info.cursor_pos {
            self.mouse_pos = Vector2::new(cursor_pos.x as f32, cursor_pos.y as f32);
        }

        // Check scrollbar hover states
        if self.needs_vertical_scrollbar() {
            let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
            let thumb_bounds = self.get_vertical_thumb_bounds(scrollbar_bounds);
            
            self.vertical_scrollbar_hovered = thumb_bounds.contains(Point::new(
                self.mouse_pos.x as f64,
                self.mouse_pos.y as f64,
            ));
        }

        if self.needs_horizontal_scrollbar() {
            let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
            let thumb_bounds = self.get_horizontal_thumb_bounds(scrollbar_bounds);
            
            self.horizontal_scrollbar_hovered = thumb_bounds.contains(Point::new(
                self.mouse_pos.x as f64,
                self.mouse_pos.y as f64,
            ));
        }

        if self.scrollbar_buttons == ScrollbarButtons::Always {
            let mouse_point = Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64);
            if self.needs_vertical_scrollbar() {
                let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
                self.up_button_hovered = self.get_vertical_up_button_bounds(scrollbar_bounds).contains(mouse_point);
                self.down_button_hovered = self.get_vertical_down_button_bounds(scrollbar_bounds).contains(mouse_point);
            }
            if self.needs_horizontal_scrollbar() {
                let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
                self.left_button_hovered = self.get_horizontal_left_button_bounds(scrollbar_bounds).contains(mouse_point);
                self.right_button_hovered = self.get_horizontal_right_button_bounds(scrollbar_bounds).contains(mouse_point);
            }
        }

        // Handle mouse wheel scrolling (GTK3-style responsiveness)
        if let Some(scroll_delta) = info.mouse_scroll_delta {
            let base_scroll_speed = 40.0; // Base scroll speed
            let (dx, dy) = match scroll_delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    // Line-based scrolling: scale based on content size for natural feel
                    let vertical_scale = (self.content_size.y / self.viewport_size.y).min(5.0).max(1.0);
                    let horizontal_scale = (self.content_size.x / self.viewport_size.x).min(5.0).max(1.0);
                    (x * base_scroll_speed * horizontal_scale, y * base_scroll_speed * vertical_scale)
                },
                MouseScrollDelta::PixelDelta(pos) => {
                    // Pixel-based scrolling: direct mapping
                    (pos.x as f32, pos.y as f32)
                },
            };
            let old_offset = self.scroll_offset();
            self.scroll_by(-dx, -dy);
            let new_offset = self.scroll_offset();
            
            // If scroll offset changed, trigger draw update only
            if old_offset != new_offset {
                update |= Update::DRAW;
            }
        }

        // Handle mouse drag scrolling
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left {
                match *state {
                    ElementState::Pressed => {
                        if self.up_button_hovered {
                            self.button_held = Some(ArrowDirection::Up);
                        } else if self.down_button_hovered {
                            self.button_held = Some(ArrowDirection::Down);
                        } else if self.left_button_hovered {
                            self.button_held = Some(ArrowDirection::Left);
                        } else if self.right_button_hovered {
                            self.button_held = Some(ArrowDirection::Right);
                        }

                        // Check if clicking on scrollbar thumbs
                        if self.needs_vertical_scrollbar() {
                            let thumb_bounds = self.get_vertical_thumb_bounds(self.get_vertical_scrollbar_bounds(layout));
                            if thumb_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64)) {
                                self.dragging_vertical = true;
                                self.drag_start_pos = self.mouse_pos;
                                self.drag_start_offset = self.scroll_offset();
                            }
                        }

                        if self.needs_horizontal_scrollbar() {
                            let thumb_bounds = self.get_horizontal_thumb_bounds(self.get_horizontal_scrollbar_bounds(layout));
                            if thumb_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64)) {
                                self.dragging_horizontal = true;
                                self.drag_start_pos = self.mouse_pos;
                                self.drag_start_offset = self.scroll_offset();
                            }
                        }
                    }
                    ElementState::Released => {
                        self.button_held = None;
                        self.dragging_vertical = false;
                        self.dragging_horizontal = false;
                    }
                }
            }
        }

        // Handle continuous scrolling when a button is held
        if let Some(direction) = self.button_held {
            let old_offset = self.scroll_offset();
            let scroll_step = 5.0; // Smaller step for smooth continuous scroll
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

        // Handle scrollbar dragging (GTK3 approach)
        if self.dragging_vertical || self.dragging_horizontal {
            let mouse_delta = self.mouse_pos - self.drag_start_pos;
            let mut new_offset = self.drag_start_offset;

            if self.dragging_vertical && self.content_size.y > self.viewport_size.y {
                let scrollbar_bounds = self.get_vertical_scrollbar_bounds(layout);
                let thumb_bounds = self.get_vertical_thumb_bounds(scrollbar_bounds);
                
                // Calculate available space for thumb movement
                let available_scrollbar_space = scrollbar_bounds.height() as f32 - thumb_bounds.height() as f32;
                let available_content_space = self.content_size.y - self.viewport_size.y;
                
                // Direct proportional mapping (more responsive)
                if available_scrollbar_space > 0.0 {
                    let scroll_ratio = mouse_delta.y / available_scrollbar_space;
                    new_offset.y = self.drag_start_offset.y + (scroll_ratio * available_content_space);
                    new_offset.y = new_offset.y.max(0.0).min(available_content_space);
                }
            }

            if self.dragging_horizontal && self.content_size.x > self.viewport_size.x {
                let scrollbar_bounds = self.get_horizontal_scrollbar_bounds(layout);
                let thumb_bounds = self.get_horizontal_thumb_bounds(scrollbar_bounds);
                
                // Calculate available space for thumb movement
                let available_scrollbar_space = scrollbar_bounds.width() as f32 - thumb_bounds.width() as f32;
                let available_content_space = self.content_size.x - self.viewport_size.x;
                
                // Direct proportional mapping (more responsive)
                if available_scrollbar_space > 0.0 {
                    let scroll_ratio = mouse_delta.x / available_scrollbar_space;
                    new_offset.x = self.drag_start_offset.x + (scroll_ratio * available_content_space);
                    new_offset.x = new_offset.x.max(0.0).min(available_content_space);
                }
            }

            let old_offset = self.scroll_offset();
            self.set_scroll_offset(new_offset);

            // If scroll offset or mouse position changed, trigger draw update only
            if old_offset != self.scroll_offset() {
                update |= Update::DRAW;
            }
        }

        // Force redraw if hover or pressed states change
        if self.dragging_vertical != self.prev_dragging_vertical ||
           self.dragging_horizontal != self.prev_dragging_horizontal ||
           self.button_held != self.prev_button_held ||
           self.up_button_hovered != self.prev_up_button_hovered ||
           self.down_button_hovered != self.prev_down_button_hovered ||
           self.left_button_hovered != self.prev_left_button_hovered ||
           self.right_button_hovered != self.prev_right_button_hovered {
            update |= Update::DRAW;
        }

        // Update previous states
        self.prev_dragging_vertical = self.dragging_vertical;
        self.prev_dragging_horizontal = self.dragging_horizontal;
        self.prev_button_held = self.button_held;
        self.prev_up_button_hovered = self.up_button_hovered;
        self.prev_down_button_hovered = self.down_button_hovered;
        self.prev_left_button_hovered = self.left_button_hovered;
        self.prev_right_button_hovered = self.right_button_hovered;

        update
    }

    fn layout_style(&self) -> StyleNode {
        let mut style = self.layout_style.get().clone();
        
        // Reserve space for scrollbars using padding
        if self.needs_vertical_scrollbar() {
            match self.vertical_scrollbar_position {
                VerticalScrollbarPosition::Left => style.padding.left = LengthPercentage::length(self.scrollbar_width),
                VerticalScrollbarPosition::Right => style.padding.right = LengthPercentage::length(self.scrollbar_width),
            }
        }
        if self.needs_horizontal_scrollbar() {
            style.padding.bottom = LengthPercentage::length(self.scrollbar_width);
        }

        StyleNode {
            style,
            children: if let Some(child) = &self.child {
                vec![child.layout_style()]
            } else {
                vec![]
            },
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
