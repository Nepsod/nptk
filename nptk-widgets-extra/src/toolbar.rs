// SPDX-License-Identifier: LGPL-3.0-only
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{
    AlignItems, Dimension, FlexDirection, LayoutNode, LayoutStyle, LengthPercentage,
    LengthPercentageAuto, StyleNode,
};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::{Affine, Line, Point, Rect, Shape, Stroke};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetChildrenExt, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::properties::ThemeProperty;
use nptk_theme::theme::Theme;

/// Configuration for toolbar border lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarBorder {
    /// No border line.
    None,
    /// Border line at the top.
    Top,
    /// Border line at the bottom.
    Bottom,
    /// Border lines at both top and bottom.
    TopAndBottom,
}

/// A toolbar widget that displays a horizontal row of tools/buttons.
pub struct Toolbar {
    children: Vec<BoxedWidget>,
    layout_style: MaybeSignal<LayoutStyle>,
    border: ToolbarBorder,
}

impl Toolbar {
    /// Create a new empty toolbar.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            layout_style: LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::percent(1.0), // Full width
                    Dimension::length(32.0), // Desktop toolbar height
                ),
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(4.0),
                    right: LengthPercentage::length(4.0),
                    top: LengthPercentage::length(2.0),
                    bottom: LengthPercentage::length(2.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: nalgebra::Vector2::new(
                    LengthPercentage::length(2.0),
                    LengthPercentage::length(0.0),
                ),
                ..Default::default()
            }
            .into(),
            border: ToolbarBorder::None, // No border by default.
        }
    }

    /// Add a child widget to the toolbar.
    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Add multiple child widgets to the toolbar.
    pub fn with_children(mut self, children: Vec<BoxedWidget>) -> Self {
        self.children.extend(children);
        self
    }

    /// Configure the toolbar border line.
    pub fn with_border_line(mut self, border: ToolbarBorder) -> Self {
        self.border = border;
        self
    }

    /// Add a separator to the toolbar.
    pub fn with_separator(self) -> Self {
        self.with_child(ToolbarSeparator::new())
    }

    /// Add a spacer (expands to fill available space) to the toolbar.
    pub fn with_spacer(self) -> Self {
        self.with_child(ToolbarSpacer::new())
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetChildrenExt for Toolbar {
    fn set_children(&mut self, children: Vec<BoxedWidget>) {
        self.children = children;
    }

    fn add_child(&mut self, child: impl Widget + 'static) {
        self.children.push(Box::new(child));
    }
}

impl WidgetLayoutExt for Toolbar {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for Toolbar {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Toolbar")
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        // Get theme colors with proper fallbacks
        let bg_color = theme
            .get_property(self.widget_id(), &ThemeProperty::ColorToolbarBackground)
            .unwrap_or_else(|| {
                // Fallback to MenuBar background color if available
                theme
                    .get_property(
                        WidgetId::new("nptk-widgets", "MenuBar"),
                        &ThemeProperty::ColorBackground,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(240, 240, 240))
            });

        let border_color = theme
            .get_property(self.widget_id(), &ThemeProperty::ColorToolbarBorder)
            .unwrap_or_else(|| {
                // Fallback to MenuBar border color if available
                theme
                    .get_property(
                        WidgetId::new("nptk-widgets", "MenuBar"),
                        &ThemeProperty::ColorBorder,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180))
            });

        let rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        // Draw background
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &rect.to_path(0.1),
        );

        // Draw borders based on configuration
        let stroke = Stroke::new(1.0);

        if self.border == ToolbarBorder::Top || self.border == ToolbarBorder::TopAndBottom {
            let top_border_line = Line::new(
                Point::new(rect.x0, rect.y0),
                Point::new(rect.x1, rect.y0),
            );
            graphics.stroke(
                &stroke,
                Affine::IDENTITY,
                &Brush::Solid(border_color),
                None,
                &top_border_line.to_path(0.1),
            );
        }

        if self.border == ToolbarBorder::Bottom || self.border == ToolbarBorder::TopAndBottom {
            let bottom_border_line = Line::new(
                Point::new(rect.x0, rect.y1),
                Point::new(rect.x1, rect.y1),
            );
            graphics.stroke(
                &stroke,
                Affine::IDENTITY,
                &Brush::Solid(border_color),
                None,
                &bottom_border_line.to_path(0.1),
            );
        }

        // Render children
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                child.render(graphics, theme, &layout.children[i], info, context.clone());
            }
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
        // Propagate render_postfix to children (for overlays, popups, etc.)
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                child.render_postfix(graphics, theme, &layout.children[i], info, context.clone());
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        let style = self.layout_style.get().clone();
        let children = self.children.iter().map(|c| c.layout_style()).collect();
        StyleNode { style, children }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        for (i, child) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                update |= child.update(&layout.children[i], context.clone(), info);
            }
        }
        update
    }
}

/// A vertical separator line for the toolbar.
pub struct ToolbarSeparator {
    layout_style: MaybeSignal<LayoutStyle>,
}

impl ToolbarSeparator {
    pub fn new() -> Self {
        Self {
            layout_style: LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::length(1.0),
                    Dimension::percent(0.7), // 70% of parent height for better visibility
                ),
                margin: nptk_core::layout::Rect::<LengthPercentageAuto> {
                    left: LengthPercentageAuto::length(3.0),
                    right: LengthPercentageAuto::length(3.0),
                    top: LengthPercentageAuto::auto(), // Auto margin for vertical centering
                    bottom: LengthPercentageAuto::auto(),
                },
                // Vertical centering handled by auto margins
                ..Default::default()
            }
            .into(),
        }
    }
}

impl Default for ToolbarSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetLayoutExt for ToolbarSeparator {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for ToolbarSeparator {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "ToolbarSeparator")
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Get separator color with proper fallback (more subtle than border)
        let color = theme
            .get_property(self.widget_id(), &ThemeProperty::ColorToolbarSeparator)
            .unwrap_or_else(|| {
                // Fallback to toolbar border color, or use a subtle default
                theme
                    .get_property(
                        WidgetId::new("nptk-widgets", "Toolbar"),
                        &ThemeProperty::ColorToolbarBorder,
                    )
                    .unwrap_or_else(|| Color::from_rgb8(180, 180, 180))
            });

        let rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(color),
            None,
            &rect.to_path(0.1),
        );
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, _layout: &LayoutNode, _context: AppContext, _info: &mut AppInfo) -> Update {
        Update::empty()
    }
}

/// A spacer that expands to fill available space in the toolbar.
pub struct ToolbarSpacer {
    layout_style: MaybeSignal<LayoutStyle>,
}

impl ToolbarSpacer {
    pub fn new() -> Self {
        Self {
            layout_style: LayoutStyle {
                flex_grow: 1.0,
                ..Default::default()
            }
            .into(),
        }
    }
}

impl Default for ToolbarSpacer {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetLayoutExt for ToolbarSpacer {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

impl Widget for ToolbarSpacer {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "ToolbarSpacer")
    }

    fn render(
        &mut self,
        _graphics: &mut dyn Graphics,
        _theme: &mut dyn Theme,
        _layout: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Spacer is invisible
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, _layout: &LayoutNode, _context: AppContext, _info: &mut AppInfo) -> Update {
        Update::empty()
    }
}

/// A button specialized for toolbars.
///
/// This is a convenience wrapper around `Button` that sets the style ID to "ToolbarButton".
///
/// # Tooltips
///
/// Tooltips can be added using the `with_tooltip()` method from the `Button` trait:
///
/// ```rust,ignore
/// ToolbarButton::new(child).with_tooltip("Tooltip text")
/// ```
pub struct ToolbarButton;

impl ToolbarButton {
    /// Create a new toolbar button with the given child widget.
    ///
    /// The returned button supports all `Button` methods including `with_tooltip()`.
    pub fn new(child: impl Widget + 'static) -> nptk_widgets::button::Button {
        use nptk_core::layout::{Dimension, LengthPercentage, LayoutStyle};
        
        nptk_widgets::button::Button::new(child)
            .with_style_id("ToolbarButton")
            .with_layout_style(LayoutStyle {
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(2.0),
                    right: LengthPercentage::length(2.0),
                    top: LengthPercentage::length(4.0),
                    bottom: LengthPercentage::length(4.0),
                },
                flex_grow: 0.0, // Don't grow beyond content size
                flex_shrink: 1.0, // Allow toolbar buttons to shrink to fit content
                flex_basis: Dimension::auto(), // Size based on content
                ..Default::default()
            })
    }

    /// Create a new toolbar button with multiple child widgets.
    ///
    /// The children will be arranged horizontally in a row with a small gap between them.
    /// This is useful for toolbar buttons that contain both an icon and text, or multiple elements.
    pub fn with_children(children: Vec<BoxedWidget>) -> nptk_widgets::button::Button {
        use nptk_core::layout::{AlignItems, FlexDirection, LengthPercentage, LayoutStyle};
        use nptk_widgets::container::Container;
        
        let container = Container::new(children)
            .with_layout_style(LayoutStyle {
                size: nalgebra::Vector2::new(
                    Dimension::auto(), // Size to content width
                    Dimension::auto(), // Size to content height
                ),
                min_size: nalgebra::Vector2::new(
                    Dimension::length(0.0), // No minimum width
                    Dimension::length(0.0), // No minimum height
                ),
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: nalgebra::Vector2::new(
                    LengthPercentage::length(2.0),
                    LengthPercentage::length(0.0),
                ),
                flex_grow: 0.0, // Don't grow beyond content size
                flex_shrink: 1.0, // Allow to shrink
                ..Default::default()
            });
        
        nptk_widgets::button::Button::new(container)
            .with_style_id("ToolbarButton")
            .with_layout_style(LayoutStyle {
                padding: nptk_core::layout::Rect::<LengthPercentage> {
                    left: LengthPercentage::length(2.0),
                    right: LengthPercentage::length(2.0),
                    top: LengthPercentage::length(4.0),
                    bottom: LengthPercentage::length(4.0),
                },
                flex_grow: 0.0, // Don't grow beyond content size
                flex_shrink: 1.0, // Allow toolbar buttons to shrink to fit content
                flex_basis: Dimension::auto(), // Size based on content
                ..Default::default()
            })
    }
}
