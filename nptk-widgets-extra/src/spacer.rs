// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, FlexDirection, LayoutNode, LayoutStyle, StyleNode, LayoutContext};
use nptk_core::vgi::Graphics;
use nptk_core::widget::Widget;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use async_trait::async_trait;

/// A spacer widget that expands to fill available space.
///
/// Similar to SwiftUI's `Spacer()`, this widget takes up all available space
/// in its parent's flex direction. It's useful for pushing widgets apart
/// or filling space in flex containers.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::spacer::Spacer;
/// use nptk_widgets_extra::flex::HStack;
/// use nptk_widgets::text::Text;
///
/// HStack::new()
///     .with_child(Text::new("Left"))
///     .with_child(Spacer::new())  // Pushes "Left" and "Right" apart
///     .with_child(Text::new("Right"))
/// ```
pub struct Spacer {
    min_size: f32,
}

impl Spacer {
    /// Create a new spacer that expands to fill available space.
    pub fn new() -> Self {
        Self { min_size: 0.0 }
    }

    /// Set the minimum size for this spacer.
    ///
    /// The spacer will expand to fill available space, but will never
    /// shrink below this minimum size.
    pub fn with_min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size;
        self
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for Spacer {
    fn render(
        &mut self,
        _graphics: &mut dyn Graphics,
        _theme: &mut dyn Theme,
        _layout: &LayoutNode,
        _info: &mut AppInfo,
        _context: AppContext,
    ) {
        // Spacer is invisible - it just takes up space
    }

    fn layout_style(&self, _context: &LayoutContext) -> StyleNode {
        StyleNode {
            style: LayoutStyle {
                size: Vector2::new(Dimension::auto(), Dimension::auto()),
                flex_grow: 1.0,  // Expand to fill available space
                flex_shrink: 1.0, // Can shrink if needed
                min_size: Vector2::new(
                    Dimension::length(self.min_size),
                    Dimension::length(self.min_size),
                ),
                ..Default::default()
            },
            children: Vec::new(),
            measure_func: None,
        }
    }

    async fn update(
        &mut self,
        _layout: &LayoutNode,
        _context: AppContext,
        _info: &mut AppInfo,
    ) -> Update {
        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets-extra", "Spacer")
    }
}
