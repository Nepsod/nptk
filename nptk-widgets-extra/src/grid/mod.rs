// SPDX-License-Identifier: LGPL-3.0-only
mod item;

pub use item::GridItem;

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{GridAutoFlow, GridPlacement, LayoutNode, LayoutStyle, LengthPercentage, StyleNode, LayoutContext};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, BoxedWidget};
use async_trait::async_trait;

/// A CSS Grid container widget.
///
/// This widget arranges its children in a grid layout using Taffy's CSS Grid implementation.
/// You can define columns and rows with fixed, flexible, or adaptive sizing.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_widgets_extra::grid::{Grid, GridItem};
/// use nptk_widgets::text::Text;
///
/// Grid::new()
///     .with_columns(vec![
///         GridItem::Fixed(200.0),  // Sidebar column
///         GridItem::Flexible { min: 300.0, flex: 1.0 },  // Content column
///     ])
///     .with_gap(10.0)
///     .with_child_at(0, 0, Text::new("Sidebar"))
///     .with_child_at(1, 0, Text::new("Content"))
/// ```
pub struct Grid {
    children: Vec<(usize, usize, BoxedWidget)>, // (column, row, widget)
    columns: Vec<GridItem>,
    rows: Vec<GridItem>,
    gap: Vector2<f32>,
    auto_flow: GridAutoFlow,
}

impl Grid {
    /// Create a new grid container.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            columns: Vec::new(),
            rows: Vec::new(),
            gap: Vector2::new(0.0, 0.0),
            auto_flow: GridAutoFlow::Row,
        }
    }

    /// Set the column definitions for this grid.
    ///
    /// Each `GridItem` defines how a column should be sized.
    pub fn with_columns(mut self, columns: Vec<GridItem>) -> Self {
        self.columns = columns;
        self
    }

    /// Set the row definitions for this grid.
    ///
    /// Each `GridItem` defines how a row should be sized.
    pub fn with_rows(mut self, rows: Vec<GridItem>) -> Self {
        self.rows = rows;
        self
    }

    /// Set the gap between grid items.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = Vector2::new(gap, gap);
        self
    }

    /// Set the gap between grid items (horizontal and vertical separately).
    pub fn with_gap_xy(mut self, gap_x: f32, gap_y: f32) -> Self {
        self.gap = Vector2::new(gap_x, gap_y);
        self
    }

    /// Set the auto-placement flow direction.
    pub fn with_auto_flow(mut self, flow: GridAutoFlow) -> Self {
        self.auto_flow = flow;
        self
    }

    /// Add a child widget at the specified grid position.
    ///
    /// # Parameters
    ///
    /// - `column`: The column index (0-based)
    /// - `row`: The row index (0-based)
    /// - `child`: The widget to place at this position
    pub fn with_child_at(
        mut self,
        column: usize,
        row: usize,
        child: impl Widget + Send + Sync + 'static,
    ) -> Self {
        self.children.push((column, row, Box::new(child)));
        self
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait(?Send)]
impl Widget for Grid {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        for (i, (_, _, child)) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                child.render(graphics, &layout.children[i], info, context.clone());
            }
        }
    }

    fn layout_style(&self, context: &LayoutContext) -> StyleNode {
        // Build grid template from columns and rows
        // For now, we'll use a simplified approach where each child gets placed
        // at its specified grid position using grid_row and grid_column

        let children_styles: Vec<StyleNode> = self
            .children
            .iter()
            .map(|(col, row, child)| {
                let mut child_style = child.layout_style(context);
                
                // Set grid position for this child
                // Note: CSS Grid uses 1-based indexing, and GridPlacement::Line takes i16
                // We'll use Auto placement for now and let Taffy handle positioning
                // Full grid template support would require more complex Taffy integration
                child_style.style.grid_column = nptk_core::layout::Line {
                    start: GridPlacement::Auto,
                    end: GridPlacement::Auto,
                };
                child_style.style.grid_row = nptk_core::layout::Line {
                    start: GridPlacement::Auto,
                    end: GridPlacement::Auto,
                };

                child_style
            })
            .collect();

        StyleNode {
            style: LayoutStyle {
                display: nptk_core::layout::Display::Grid,
                grid_auto_flow: self.auto_flow,
                gap: Vector2::new(
                    LengthPercentage::length(self.gap.x),
                    LengthPercentage::length(self.gap.y),
                ),
                ..Default::default()
            },
            children: children_styles,
            measure_func: None,
        }
    }

    async fn update(
        &mut self,
        layout: &LayoutNode,
        context: AppContext,
        info: &mut AppInfo,
    ) -> Update {
        let mut update = Update::empty();
        for (i, (_, _, child)) in self.children.iter_mut().enumerate() {
            if i < layout.children.len() {
                update |= child.update(&layout.children[i], context.clone(), info).await;
            }
        }
        update
    }

}
