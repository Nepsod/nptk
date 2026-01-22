use std::sync::Arc;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::layout::{LayoutNode, StyleNode, LayoutStyle, Dimension};
use nptk_core::model::{ItemModel, ItemRole, ModelData, Orientation};
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::vgi::{Graphics};
use nptk_core::vg::peniko::{Brush, Color};
use nptk_core::vg::kurbo::{Rect, Affine, Shape};
use nptk_core::text_render::TextRenderContext;
use async_trait::async_trait;
use nptk_core::signal::MaybeSignal;
use nptk_core::signal::state::StateSignal;
use nptk_core::theme::{Palette, ColorRole};
use nptk_core::window::{MouseButton, ElementState};

/// View mode for the ItemView
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Icon,
    Table,
}

pub struct ItemView {
    model: Arc<dyn ItemModel>,
    view_mode: ViewMode,
    layout_style: MaybeSignal<LayoutStyle>,
    item_height: f32,
    text_context: TextRenderContext,
    selected_rows: MaybeSignal<Vec<usize>>,
    on_selection_change: Option<Box<dyn Fn(Vec<usize>) -> Update + Send + Sync>>,
}

impl ItemView {
    pub fn new(model: Arc<dyn ItemModel>) -> Self {
        Self {
            model,
            view_mode: ViewMode::List,
            layout_style: Default::default(),
            item_height: 30.0,
            text_context: TextRenderContext::new(),
            selected_rows: MaybeSignal::signal(Box::new(StateSignal::new(Vec::new()))),
            on_selection_change: None,
        }
    }
    
    pub fn with_selected_rows(mut self, signal: impl Into<MaybeSignal<Vec<usize>>>) -> Self {
        self.selected_rows = signal.into();
        self
    }
    
    pub fn with_on_selection_change<F>(mut self, callback: F) -> Self 
    where F: Fn(Vec<usize>) -> Update + Send + Sync + 'static 
    {
        self.on_selection_change = Some(Box::new(callback));
        self
    }

    pub fn with_view_mode(mut self, mode: ViewMode) -> Self {
        self.view_mode = mode;
        self
    }
    
    pub fn selected_rows_signal(&self) -> &MaybeSignal<Vec<usize>> {
        &self.selected_rows
    }
    
    fn render_list(&mut self, graphics: &mut dyn Graphics, layout_node: &LayoutNode, info: &mut AppInfo, context: &AppContext) {
         let rows = self.model.row_count();
         // Basic rendering logic (can be optimized)
         // Assuming layout_node.layout provides correct size
         
         let start_y = layout_node.layout.location.y;
         let mut y = start_y;
         
         let palette = context.palette();
         
         for i in 0..rows {
            let is_selected = self.selected_rows.get().contains(&i);
            if y > start_y + layout_node.layout.size.height {
                break;
            }
            if y + self.item_height < start_y {
                y += self.item_height;
                continue;
            }
            
            let row_rect = Rect::new(
                layout_node.layout.location.x as f64,
                y as f64,
                (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                (y + self.item_height) as f64,
            );
            
             // Fetch data
            if is_selected {
                let selection_color = palette.color(ColorRole::Selection);
                graphics.fill(
                     nptk_core::vg::peniko::Fill::NonZero,
                     Affine::IDENTITY,
                     &Brush::Solid(selection_color),
                     None,
                     &row_rect.to_path(0.1)
                );
            }
            
            let text_data = self.model.data(i, 0, ItemRole::Display);
            if let ModelData::String(text) = text_data {
                let text_color = palette.color(ColorRole::BaseText);
                let transform = Affine::translate((row_rect.x0 + 5.0, row_rect.y0 + 5.0));
                
                 self.text_context.render_text(
                    &mut info.font_context,
                    graphics,
                    &text,
                    None,
                    16.0,
                    Brush::Solid(text_color),
                    transform,
                    true,
                    Some(row_rect.width() as f32 - 10.0),
                );
            }
            
            y += self.item_height;
         }
    }

    fn render_table(&mut self, graphics: &mut dyn Graphics, layout_node: &LayoutNode, info: &mut AppInfo, context: &AppContext) {
         let rows = self.model.row_count();
         let cols = self.model.column_count();
         let start_y = layout_node.layout.location.y + 30.0; // Header height
         let mut y = start_y;
         
         let palette = context.palette();
         let header_bg = palette.color(ColorRole::Window).with_alpha(0.8);
         let border_color = palette.color(ColorRole::ThreedShadow1);
         
         // Draw Header
         let header_rect = Rect::new(
            layout_node.layout.location.x as f64,
            layout_node.layout.location.y as f64,
            (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
            (layout_node.layout.location.y + 30.0) as f64
         );
         
         graphics.fill(
             nptk_core::vg::peniko::Fill::NonZero,
             Affine::IDENTITY,
             &Brush::Solid(header_bg),
             None,
             &header_rect.to_path(0.1)
        );
        
        let col_width = (layout_node.layout.size.width / cols as f32) as f64;
        
        for c in 0..cols {
             let x = layout_node.layout.location.x as f64 + (c as f64 * col_width);
             
             // Header Text
             if let ModelData::String(header_text) = self.model.header_data(c, Orientation::Horizontal, ItemRole::Display) {
                 let text_color = palette.color(ColorRole::BaseText);
                 let transform = Affine::translate((x + 5.0, layout_node.layout.location.y as f64 + 5.0));
                 
                 self.text_context.render_text(
                        &mut info.font_context,
                        graphics,
                        &header_text,
                        None,
                        14.0, // Header font size
                        Brush::Solid(text_color),
                        transform,
                        true,
                        Some(col_width as f32 - 10.0),
                    );
             }
             
             // Separator
             if c > 0 {
                  let sep_path = nptk_core::vg::kurbo::Line::new(
                      (x, layout_node.layout.location.y as f64),
                      (x, (layout_node.layout.location.y + 30.0) as f64)
                  );
                   graphics.stroke(
                         &nptk_core::vg::kurbo::Stroke::new(1.0),
                         Affine::IDENTITY,
                         &Brush::Solid(border_color),
                         None,
                         &sep_path.to_path(0.1)
                    );
             }
        }
         
         for i in 0..rows {
            let is_selected = self.selected_rows.get().contains(&i);
            if y > start_y + layout_node.layout.size.height {
                break;
            }
            if y + self.item_height < start_y {
                y += self.item_height;
                continue;
            }
            
            let row_rect = Rect::new(
                layout_node.layout.location.x as f64,
                y as f64,
                (layout_node.layout.location.x + layout_node.layout.size.width) as f64,
                (y + self.item_height) as f64,
            );
            
            if is_selected {
                let selection_color = palette.color(ColorRole::Selection);
                graphics.fill(
                     nptk_core::vg::peniko::Fill::NonZero,
                     Affine::IDENTITY,
                     &Brush::Solid(selection_color),
                     None,
                     &row_rect.to_path(0.1)
                );
            }
            
            // Draw cells
            for c in 0..cols {
                 let x = layout_node.layout.location.x as f64 + (c as f64 * col_width);
                 let cell_rect = Rect::new(
                    x,
                    y as f64,
                    x + col_width,
                    (y + self.item_height) as f64
                 );
                 
                  // Fetch data
                let text_data = self.model.data(i, c, ItemRole::Display);
                if let ModelData::String(text) = text_data {
                    let text_color = palette.color(ColorRole::BaseText);
                    let transform = Affine::translate((cell_rect.x0 + 5.0, cell_rect.y0 + 5.0));
                    
                     self.text_context.render_text(
                        &mut info.font_context,
                        graphics,
                        &text,
                        None,
                        16.0,
                        Brush::Solid(text_color),
                        transform,
                        true,
                        Some(cell_rect.width() as f32 - 10.0),
                    );
                }
            }
            
            y += self.item_height;
         }
    }
}

#[async_trait(?Send)]
impl Widget for ItemView {
    fn layout_style(&self, _context: &nptk_core::layout::LayoutContext) -> StyleNode {
         StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![], // Leaf for now (virtual scrolling usually manages children manually or renders directly)
            measure_func: None,
        }
    }

    async fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        
        // Handle Mouse Events for Selection
        if let Some(pos) = info.cursor_pos {
            let rect = layout.layout;
            if pos.x >= rect.location.x as f64 && pos.x <= (rect.location.x + rect.size.width) as f64 &&
               pos.y >= rect.location.y as f64 && pos.y <= (rect.location.y + rect.size.height) as f64 {
                // Check for clicks
                let left_click = info.buttons.iter().any(|(_, btn, state)| *btn == MouseButton::Left && *state == ElementState::Pressed);
                if left_click {
                    // Calculate clicked row
                     let item_y = if self.view_mode == ViewMode::Table {
                        pos.y - (layout.layout.location.y as f64) - 30.0 // Minus header
                    } else {
                        pos.y - (layout.layout.location.y as f64)
                    };
                    
                    if item_y >= 0.0 {
                        let row_index = (item_y / self.item_height as f64).floor() as usize;
                        if row_index < self.model.row_count() {
                            // Select this row
                            // TODO: Add support for multi-selection (Ctrl/Shift)
                            // For now, simple single selection or toggle
                            let mut current_selection = self.selected_rows.get().clone();
                            
                            // Simple clear and select logic for now
                            current_selection.clear();
                            current_selection.push(row_index);
                            
                            if let Some(signal) = self.selected_rows.as_signal() {
                                signal.set(current_selection.clone());
                            }
                            
                            if let Some(cb) = &self.on_selection_change {
                                update |= cb(current_selection);
                            }
                            
                            update.insert(Update::DRAW);
                        }
                    }
                }
            }
        }
        
        update
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        layout: &LayoutNode,
        info: &mut AppInfo,
        context: AppContext,
    ) {
        let rows = self.model.row_count();
        if rows == 0 {
            return;
        }

        let palette = context.palette();
        let bg_color = palette.color(ColorRole::Base); // or specific view bg
        
        let rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );

        // Draw background
        graphics.fill(
             nptk_core::vg::peniko::Fill::NonZero,
             Affine::IDENTITY,
             &Brush::Solid(bg_color),
             None,
             &rect.to_path(0.1)
        );

        // Render based on mode
        match self.view_mode {
            ViewMode::List => self.render_list(graphics, layout, info, &context),
            ViewMode::Table => self.render_table(graphics, layout, info, &context),
            _ => self.render_list(graphics, layout, info, &context), // Fallback
        }
    }
}

impl WidgetLayoutExt for ItemView {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
