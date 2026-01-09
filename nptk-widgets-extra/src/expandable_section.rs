// SPDX-License-Identifier: LGPL-3.0-only
//! Expandable section widget for collapsible content areas.
//!
//! This widget provides a header that can be clicked
//! to expand or collapse the content below.

use nptk_widgets::icon::Icon;
use nptk_widgets::text::Text;
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, Display, LayoutNode, LayoutStyle, LengthPercentage, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::sync::Arc;

/// An expandable section widget with a clickable header and collapsible content.
///
/// The header displays a title, optional icon, and an expand/collapse indicator.
/// Clicking the header toggles the expanded state, showing or hiding the content.
///
/// ### Theming
/// The header can be styled with:
/// - `color_background` - Background color of the header
/// - `color_text` - Text color for the title
/// - `color_hovered` - Background color when hovering over the header
pub struct ExpandableSection {
    // Header content
    title: String,
    icon: Option<String>,
    
    // State
    expanded: StateSignal<bool>,
    
    // Content
    content: BoxedWidget,
    
    // Interaction
    on_expanded_changed: Option<Arc<dyn Fn(bool) -> Update + Send + Sync>>,
    
    // Layout
    layout_style: MaybeSignal<LayoutStyle>,
    
    // Styling
    header_height: f32,
    icon_size: u32,
    indicator_size: u32,
    
    // Internal state
    hovered: bool,
    text_render_context: TextRenderContext,
}

impl ExpandableSection {
    /// Create a new expandable section with the given title and content.
    ///
    /// # Arguments
    /// * `title` - The title text displayed in the header
    /// * `content` - The widget to display when expanded
    pub fn new(title: impl Into<String>, content: impl Widget + 'static) -> Self {
        Self {
            title: title.into(),
            icon: None,
            expanded: StateSignal::new(false),
            content: Box::new(content),
            on_expanded_changed: None,
            layout_style: LayoutStyle {
                flex_direction: nptk_core::layout::FlexDirection::Column,
                ..Default::default()
            }
            .into(),
            header_height: 32.0,
            icon_size: 16,
            indicator_size: 16,
            hovered: false,
            text_render_context: TextRenderContext::new(),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Set an optional icon for the header.
    pub fn with_icon(self, icon_name: impl Into<String>) -> Self {
        self.apply_with(|s| s.icon = Some(icon_name.into()))
    }

    /// Set the initial expanded state.
    pub fn with_expanded(self, expanded: bool) -> Self {
        self.apply_with(|s| {
            s.expanded.set(expanded);
        })
    }

    /// Set a callback to be called when the expanded state changes.
    pub fn with_on_expanded_changed(
        self,
        callback: impl Fn(bool) -> Update + Send + Sync + 'static,
    ) -> Self {
        self.apply_with(|s| s.on_expanded_changed = Some(Arc::new(callback)))
    }

    /// Set the header height in pixels.
    pub fn with_header_height(self, height: f32) -> Self {
        self.apply_with(|s| s.header_height = height)
    }

    /// Set the icon size in pixels.
    pub fn with_icon_size(self, size: u32) -> Self {
        self.apply_with(|s| s.icon_size = size)
    }

    /// Set the indicator (arrow) icon size in pixels.
    pub fn with_indicator_size(self, size: u32) -> Self {
        self.apply_with(|s| s.indicator_size = size)
    }

    /// Get a reference to the expanded state signal for external control.
    pub fn expanded_signal(&self) -> &StateSignal<bool> {
        &self.expanded
    }

    /// Toggle the expanded state.
    fn toggle(&mut self) -> Update {
        let current_state = *self.expanded.get();
        let new_state = !current_state;
        self.expanded.set(new_state);
        
        let mut update = Update::DRAW;
        if let Some(ref callback) = self.on_expanded_changed {
            update |= callback(new_state);
        }
        update
    }

    /// Check if a point is within the header bounds.
    fn is_in_header_bounds(&self, layout: &LayoutNode, x: f32, y: f32) -> bool {
        let header_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + self.header_height) as f64,
        );
        header_rect.contains((x as f64, y as f64))
    }

    /// Render the header with title, optional icon, and expand/collapse indicator.
    fn render_header(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        let header_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + self.header_height) as f64,
        );

        // Draw background
        let bg_color = if self.hovered {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorHovered,
                )
                .or_else(|| {
                    theme.get_default_property(
                        &nptk_theme::properties::ThemeProperty::ColorHovered,
                    )
                })
                .unwrap_or_else(|| Color::from_rgb8(240, 240, 240))
        } else {
            theme
                .get_property(
                    self.widget_id(),
                    &nptk_theme::properties::ThemeProperty::ColorBackground,
                )
                .or_else(|| {
                    theme.get_default_property(
                        &nptk_theme::properties::ThemeProperty::ColorBackground,
                    )
                })
                .unwrap_or_else(|| theme.window_background())
        };

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &header_rect.to_path(0.1),
        );

        // Calculate positions
        let padding = 8.0f32;
        let mut x = layout.layout.location.x + padding;
        let y = layout.layout.location.y + self.header_height / 2.0;

        // Render optional icon
        if let Some(ref icon_name) = self.icon {
            let icon_size_f = self.icon_size as f32;
            let icon_y = y - icon_size_f / 2.0;
            
            // Create a temporary layout node for the icon
            let icon_layout = LayoutNode {
                layout: nptk_core::layout::Layout {
                    location: nptk_core::layout::Layout::default().location, // Will set via transform
                    size: nptk_core::layout::Layout::default().size, // Will set via transform
                    ..Default::default()
                },
                children: vec![],
            };
            
            // Use graphics transform to position the icon
            let icon_widget = Icon::new(icon_name.clone(), self.icon_size, None);
            let icon_transform = Affine::translate(Vec2::new(x as f64, icon_y as f64));
            
            // Create a scene for the icon and render it with transform
            if let Some(scene) = graphics.as_scene_mut() {
                let mut icon_scene = nptk_core::vg::Scene::new();
                let mut icon_gfx = nptk_core::vgi::vello_vg::VelloGraphics::new(&mut icon_scene);
                let mut icon_widget = icon_widget;
                icon_widget.render(&mut icon_gfx, theme, &icon_layout, info, context.clone());
                scene.append(&icon_scene, Some(icon_transform));
            }
            
            x += icon_size_f + padding;
        }

        // Render title text
        let text_color = theme
            .get_property(
                self.widget_id(),
                &nptk_theme::properties::ThemeProperty::ColorText,
            )
            .or_else(|| {
                theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText)
            })
            .unwrap_or_else(|| Color::from_rgb8(0, 0, 0));

        let text_width = layout.layout.size.width - (x - layout.layout.location.x) - self.indicator_size as f32 - padding * 2.0;
        let text_widget = Text::new(self.title.clone())
            .with_font_size(14.0)
            .with_layout_style(LayoutStyle {
                size: Vector2::new(
                    Dimension::length(text_width.max(0.0)),
                    Dimension::length(self.header_height),
                ),
                ..Default::default()
            });

        // Render text using graphics transform
        let mut text_scene = nptk_core::vg::Scene::new();
        let mut text_graphics = nptk_core::vgi::vello_vg::VelloGraphics::new(&mut text_scene);
        let text_layout = LayoutNode {
            layout: nptk_core::layout::Layout::default(),
            children: vec![],
        };
        let mut text_widget = text_widget;
        text_widget.render(&mut text_graphics, theme, &text_layout, info, context.clone());
        
        // Apply transform to position the text
        if let Some(ref mut scene) = graphics.as_scene_mut() {
            let transform = Affine::translate(Vec2::new(x as f64, layout.layout.location.y as f64));
            scene.append(&text_scene, Some(transform));
        }

        // Render expand/collapse indicator
        let expanded_state = *self.expanded.get();
        let indicator_icon = if expanded_state {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        };

        let indicator_x = layout.layout.location.x + layout.layout.size.width
            - self.indicator_size as f32
            - padding;
        let indicator_y = y - self.indicator_size as f32 / 2.0;

        // Render indicator using graphics transform
        let mut indicator_scene = nptk_core::vg::Scene::new();
        let mut indicator_graphics = nptk_core::vgi::vello_vg::VelloGraphics::new(&mut indicator_scene);
        let indicator_layout = LayoutNode {
            layout: nptk_core::layout::Layout::default(),
            children: vec![],
        };
        let mut indicator_widget = Icon::new(indicator_icon, self.indicator_size, None);
        indicator_widget.set_layout_style(LayoutStyle {
            size: Vector2::new(
                Dimension::length(self.indicator_size as f32),
                Dimension::length(self.indicator_size as f32),
            ),
            ..Default::default()
        });
        indicator_widget.render(&mut indicator_graphics, theme, &indicator_layout, info, context.clone());
        
        // Apply transform to position the indicator
        if let Some(ref mut scene) = graphics.as_scene_mut() {
            let transform = Affine::translate(Vec2::new(indicator_x as f64, indicator_y as f64));
            scene.append(&indicator_scene, Some(transform));
        }
    }
}

impl WidgetLayoutExt for ExpandableSection {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for ExpandableSection {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "ExpandableSection")
    }

    fn layout_style(&self) -> StyleNode {
        let mut style = self.layout_style.get().clone();
        style.flex_direction = nptk_core::layout::FlexDirection::Column;

        // Create header style (fixed height)
        let header_style = LayoutStyle {
            size: Vector2::new(
                Dimension::percent(100.0),
                Dimension::length(self.header_height),
            ),
            flex_direction: nptk_core::layout::FlexDirection::Row,
            align_items: Some(nptk_core::layout::AlignItems::Center),
            padding: nptk_core::layout::Rect {
                left: LengthPercentage::length(8.0),
                right: LengthPercentage::length(8.0),
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(0.0),
            },
            ..Default::default()
        };

        // Create content style (conditional display)
        let expanded_state = *self.expanded.get();
        let content_display = if expanded_state {
            Display::Flex
        } else {
            Display::None
        };

        let content_style = LayoutStyle {
            display: content_display,
            flex_grow: 1.0,
            ..Default::default()
        };

        StyleNode {
            style,
            children: vec![
                StyleNode {
                    style: header_style,
                    children: vec![],
                },
                StyleNode {
                    style: content_style,
                    children: vec![self.content.layout_style()],
                },
            ],
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();

        // Update content first
        let expanded_state = *self.expanded.get();
        if expanded_state && layout.children.len() > 1 {
            update.insert(
                self.content
                    .update(&layout.children[1], context.clone(), info),
            );
        }

        // Handle header clicks
        if let Some(cursor) = info.cursor_pos {
            let cursor_x = cursor.x as f32;
            let cursor_y = cursor.y as f32;
            let in_header = self.is_in_header_bounds(layout, cursor_x, cursor_y);

            // Update hover state
            self.hovered = in_header;

            // Check for clicks
            for (_, button, state) in &info.buttons {
                if *button == MouseButton::Left && *state == ElementState::Released {
                    if in_header {
                        update.insert(self.toggle());
                    }
                }
            }
        } else {
            self.hovered = false;
        }

        update
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Render header (always visible)
        if layout.children.len() > 0 {
            self.render_header(graphics, theme, &layout.children[0], info, context.clone());
        }

        // Render content (only when expanded)
        if *self.expanded.get() && layout.children.len() > 1 {
            self.content
                .render(graphics, theme, &layout.children[1], info, context);
        }
    }
}
