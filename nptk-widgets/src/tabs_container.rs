use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{MaybeSignal, Signal, state::StateSignal};
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke, Point};
use nptk_core::vg::peniko::{Fill, Color};
use nptk_core::vg::Scene;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use nalgebra::Vector2;
use std::sync::Arc;

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
    pub fn new(id: impl Into<String>, label: impl Into<String>, content: impl Widget + 'static) -> Self {
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
    pub fn with_close_callback(mut self, callback: impl Fn() -> Update + Send + Sync + 'static) -> Self {
        self.on_close = Some(Arc::new(callback));
        self
    }
}

/// A container widget that displays tabs and switches between content
pub struct TabsContainer {
    /// Widget ID
    widget_id: WidgetId,
    /// Layout style
    layout_style: MaybeSignal<LayoutStyle>,
    /// List of tabs
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
}

impl TabsContainer {
    /// Create a new TabsContainer
    pub fn new() -> Self {
        Self {
            widget_id: WidgetId::new("tabs_container", "default"),
            layout_style: MaybeSignal::value(LayoutStyle::default()),
            tabs: Vec::new(),
            active_tab: StateSignal::new(0),
            tab_position: TabPosition::Top,
            tab_size: 32.0,
            mouse_pos: Vector2::zeros(),
            hovered_tab: None,
            pressed_tab: None,
            hovered_close: None,
        }
    }

    /// Add a tab to the container
    pub fn add_tab(&mut self, tab: TabItem) {
        self.tabs.push(tab);
    }

    /// Add a tab to the container (builder pattern)
    pub fn with_tab(mut self, tab: TabItem) -> Self {
        self.add_tab(tab);
        self
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
        if index < self.tabs.len() {
            self.active_tab.set(index);
        }
    }

    /// Get tab bounds for the given index within the tab bar area
    fn get_tab_bounds(&self, layout: &LayoutNode, index: usize) -> Rect {
        let tab_bar_bounds = self.get_tab_bar_bounds(layout);
        let tab_count = self.tabs.len();
        
        if tab_count == 0 || index >= tab_count {
            return Rect::ZERO;
        }

        match self.tab_position {
            TabPosition::Top | TabPosition::Bottom => {
                // Horizontal tabs - distribute evenly across tab bar width
                let tab_width = tab_bar_bounds.width() / tab_count as f64;
                let tab_x = tab_bar_bounds.x0 + (index as f64 * tab_width);

                Rect::new(
                    tab_x,
                    tab_bar_bounds.y0,
                    tab_x + tab_width,
                    tab_bar_bounds.y1,
                )
            }
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
            }
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


    /// Render text on a tab
    fn render_text(&self, _scene: &mut Scene, _text: &str, _x: f64, _y: f64, _color: Color, _info: &AppInfo) {
        let _font_size = 14.0;
        // Use approximate character width for text measurement
        // TODO: Implement proper text measurement when needed
        
        // TODO: Fix the FileRef lifetime issue
        // let location = font_ref.axes().location::<&[VariationSetting; 0]>(&[]);
        // let glyph_metrics = font_ref.glyph_metrics(Size::new(font_size), &location);
        // let charmap = font_ref.charmap();

        // TODO: Implement proper text rendering
        // For now, text rendering is handled by the TextRenderContext
    }
}

impl Widget for TabsContainer {
    fn render(
        &mut self,
        scene: &mut Scene,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        let widget_theme = theme.of(self.widget_id());
        
        // Draw container background
        let container_bounds = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        if let Some(ref theme) = widget_theme {
            if let Some(bg_color) = theme.get_color("background") {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &bg_color,
                    None,
                    &container_bounds,
                );
            }
        }

        // Draw tab bar background (slightly different color)
        let tab_bar_bounds = self.get_tab_bar_bounds(layout);
        let tab_bar_color = if let Some(ref theme) = widget_theme {
            theme.get_color("tab_bar_background").unwrap_or(Color::from_rgb8(250, 250, 250))
        } else {
            Color::from_rgb8(250, 250, 250)
        };

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &tab_bar_color,
            None,
            &tab_bar_bounds,
        );

        // Draw tab bar border
        let border_color = if let Some(ref theme) = widget_theme {
            theme.get_color("border").unwrap_or(Color::from_rgb8(180, 180, 180))
        } else {
            Color::from_rgb8(180, 180, 180)
        };
        
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &border_color,
            None,
            &tab_bar_bounds,
        );

        // Draw tabs
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_bounds = self.get_tab_bounds(layout, index);
            let is_active = index == self.active_tab();
            let is_hovered = self.hovered_tab == Some(index);
            let is_pressed = self.pressed_tab == Some(index);

            // Tab background with rounded corners
            let tab_color = if let Some(ref theme) = widget_theme {
                if is_active {
                    theme.get_color("tab_active").unwrap_or(Color::WHITE)
                } else if is_pressed {
                    theme.get_color("tab_pressed").unwrap_or(Color::from_rgb8(200, 200, 200))
                } else if is_hovered {
                    theme.get_color("tab_hovered").unwrap_or(Color::from_rgb8(220, 220, 220))
                } else {
                    theme.get_color("tab_inactive").unwrap_or(Color::from_rgb8(230, 230, 230))
                }
            } else {
                Color::from_rgb8(230, 230, 230)
            };

            // Create rounded rectangle for tab (only round top corners for top tabs)
            let tab_rounded = match self.tab_position {
                TabPosition::Top => RoundedRect::from_rect(
                    tab_bounds, 
                    RoundedRectRadii::new(6.0, 6.0, 0.0, 0.0)
                ),
                TabPosition::Bottom => RoundedRect::from_rect(
                    tab_bounds, 
                    RoundedRectRadii::new(0.0, 0.0, 6.0, 6.0)
                ),
                TabPosition::Left => RoundedRect::from_rect(
                    tab_bounds, 
                    RoundedRectRadii::new(6.0, 0.0, 0.0, 6.0)
                ),
                TabPosition::Right => RoundedRect::from_rect(
                    tab_bounds, 
                    RoundedRectRadii::new(0.0, 6.0, 6.0, 0.0)
                ),
            };

            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &tab_color,
                None,
                &tab_rounded,
            );

            // Tab border with subtle styling
            let border_color = if let Some(ref theme) = widget_theme {
                theme.get_color("border").unwrap_or(Color::from_rgb8(180, 180, 180))
            } else {
                Color::from_rgb8(180, 180, 180)
            };
            
            scene.stroke(
                &Stroke::new(if is_active { 1.5 } else { 1.0 }),
                Affine::IDENTITY,
                &border_color,
                None,
                &tab_rounded,
            );

            // Tab text with proper rendering and theme colors
            let text_color = if let Some(ref theme) = widget_theme {
                if is_active {
                    theme.get_color("tab_text_active").unwrap_or(Color::BLACK)
                } else {
                    theme.get_color("tab_text").unwrap_or(Color::from_rgb8(50, 50, 50))
                }
            } else {
                if is_active { Color::BLACK } else { Color::from_rgb8(50, 50, 50) }
            };

            // Center text in tab
            let text_x = tab_bounds.x0 + 10.0; // Left padding
            let text_y = tab_bounds.y0 + (tab_bounds.height() - 14.0) / 2.0; // Center vertically
            
            self.render_text(scene, &tab.label, text_x, text_y, text_color, _info);
            
            // Close button if available
            if tab.on_close.is_some() {
                let close_bounds = self.get_close_button_bounds(tab_bounds);
                let close_hovered = self.hovered_close == Some(index);
                
                let close_color = if close_hovered {
                    Color::from_rgb8(255, 100, 100)
                } else {
                    Color::from_rgb8(150, 150, 150)
                };

                // Draw X for close button (draw two lines to form an X)
                let close_center_x = close_bounds.center().x;
                let close_center_y = close_bounds.center().y;
                let close_size = 6.0;
                
                // Draw the X lines
                scene.stroke(
                    &Stroke::new(2.0),
                    Affine::IDENTITY,
                    &close_color,
                    None,
                    &Rect::new(
                        close_center_x - close_size / 2.0,
                        close_center_y - close_size / 2.0,
                        close_center_x + close_size / 2.0,
                        close_center_y + close_size / 2.0,
                    ),
                );
            }
        }

        // Render active tab content in the content area
        let active_tab_index = self.active_tab();
        let content_bounds = self.get_content_bounds(layout);
        
        if let Some(active_tab) = self.tabs.get_mut(active_tab_index) {
            // Draw content area background
            let content_bg_color = if let Some(ref theme) = widget_theme {
                theme.get_color("content_background").unwrap_or(Color::WHITE)
            } else {
                Color::WHITE
            };

            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &content_bg_color,
                None,
                &content_bounds,
            );

            // Draw content area border
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &border_color,
                None,
                &content_bounds,
            );

            // Render content directly in the content area using child layout if available
            if !layout.children.is_empty() {
                // Use the first child's layout (which should be positioned correctly)
                active_tab.content.render(scene, theme, &layout.children[0], _info, _context);
            } else {
                // Fallback: just render with original layout (content might overlap tabs)
                active_tab.content.render(scene, theme, layout, _info, _context);
            }
        }
    }

    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Update mouse position
        if let Some(cursor_pos) = info.cursor_pos {
            self.mouse_pos = Vector2::new(cursor_pos.x as f32, cursor_pos.y as f32);
        }

        // Check tab hover states
        self.hovered_tab = None;
        self.hovered_close = None;
        
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_bounds = self.get_tab_bounds(layout, index);
            
            if tab_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64)) {
                self.hovered_tab = Some(index);
                
                // Check close button hover if tab has close button
                if tab.on_close.is_some() {
                    let close_bounds = self.get_close_button_bounds(tab_bounds);
                    if close_bounds.contains(Point::new(self.mouse_pos.x as f64, self.mouse_pos.y as f64)) {
                        self.hovered_close = Some(index);
                    }
                }
                break;
            }
        }

        // Handle mouse clicks
        for (_, button, state) in &info.buttons {
            if *button == MouseButton::Left {
                match *state {
                    ElementState::Pressed => {
                        if let Some(hovered_tab) = self.hovered_tab {
                            self.pressed_tab = Some(hovered_tab);
                            
                            // Check if clicking close button
                            if self.hovered_close == Some(hovered_tab) {
                                if let Some(ref callback) = self.tabs[hovered_tab].on_close {
                                    update |= callback();
                                }
                            } else {
                                // Switch to clicked tab
                                self.set_active_tab(hovered_tab);
                                update |= Update::DRAW;
                            }
                        }
                    }
                    ElementState::Released => {
                        self.pressed_tab = None;
                    }
                }
            }
        }

        // Update active tab content
        let active_tab_index = self.active_tab();
        let _content_bounds = self.get_content_bounds(layout);
        if let Some(active_tab) = self.tabs.get_mut(active_tab_index) {
            
            // Use the parent layout for content rendering (simplified approach)
            let content_layout = layout;

            update |= active_tab.content.update(&content_layout, _context, info);
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
                        child_style.style.margin.bottom = LengthPercentageAuto::length(self.tab_size);
                    },
                    TabPosition::Left => {
                        child_style.style.margin.left = LengthPercentageAuto::length(self.tab_size);
                    },
                    TabPosition::Right => {
                        child_style.style.margin.right = LengthPercentageAuto::length(self.tab_size);
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
