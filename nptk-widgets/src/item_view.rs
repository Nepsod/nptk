use std::sync::Arc;
use std::time::{Instant, Duration};
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_core::layout::{LayoutNode, StyleNode, LayoutStyle, Dimension};
use nptk_core::model::{ItemModel, ItemRole, ModelData, Orientation, SortOrder};
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
    Compact,
}

pub struct ItemView {
    model: Arc<dyn ItemModel>,
    view_mode: MaybeSignal<ViewMode>,
    layout_style: MaybeSignal<LayoutStyle>,
    item_height: f32,
    text_context: TextRenderContext,
    selected_rows: MaybeSignal<Vec<usize>>,
    on_selection_change: Option<Box<dyn Fn(Vec<usize>) -> Update + Send + Sync>>,
    on_activate: Option<Box<dyn Fn(usize) -> Update + Send + Sync>>,
    sorted_column: MaybeSignal<Option<(usize, SortOrder)>>,
    last_click: Option<(usize, Instant)>,
    last_selected_index: Option<usize>,
    was_left_down: bool,
}

impl ItemView {
    pub fn new(model: Arc<dyn ItemModel>) -> Self {
        Self {
            model,
            view_mode: MaybeSignal::value(ViewMode::List),
            layout_style: Default::default(),
            item_height: 30.0,
            text_context: TextRenderContext::new(),
            selected_rows: MaybeSignal::signal(Box::new(StateSignal::new(Vec::new()))),
            on_selection_change: None,
            on_activate: None,
            sorted_column: MaybeSignal::value(None),
            last_click: None,
            last_selected_index: None,
            was_left_down: false,
        }
    }

    pub fn with_on_activate(mut self, callback: impl Fn(usize) -> Update + Send + Sync + 'static) -> Self {
        self.on_activate = Some(Box::new(callback));
        self
    }

    pub fn with_sorted_column(mut self, signal: impl Into<MaybeSignal<Option<(usize, SortOrder)>>>) -> Self {
        self.sorted_column = signal.into();
        self
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

    pub fn with_view_mode(mut self, mode: impl Into<MaybeSignal<ViewMode>>) -> Self {
        self.view_mode = mode.into();
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
            
             // Draw Icon
             let mut text_offset = 5.0;
             let icon_size = 16.0;

             if let ModelData::String(icon_name) = self.model.data(i, 0, ItemRole::Icon) {
                 let icon_rect = Rect::new(
                     row_rect.x0 + 5.0,
                     row_rect.y0 + (self.item_height as f64 - icon_size) / 2.0,
                     row_rect.x0 + 5.0 + icon_size,
                     row_rect.y0 + (self.item_height as f64 + icon_size) / 2.0
                 );
                 self.draw_icon(graphics, &icon_name, icon_rect, &palette);
                 text_offset += icon_size + 5.0;
             }
            
            let text_data = self.model.data(i, 0, ItemRole::Display);
            if let ModelData::String(text) = text_data {
                let text_color = palette.color(ColorRole::BaseText);
                let transform = Affine::translate((row_rect.x0 + text_offset, row_rect.y0 + 5.0));
                
                 self.text_context.render_text(
                    &mut info.font_context,
                    graphics,
                    &text,
                    None,
                    16.0,
                    Brush::Solid(text_color),
                    transform,
                    true,
                    Some(row_rect.width() as f32 - text_offset as f32 - 5.0),
                );
            }
            
            y += self.item_height;
         }
    }

    fn render_icon(&mut self, graphics: &mut dyn Graphics, layout_node: &LayoutNode, info: &mut AppInfo, context: &AppContext) {
         let rows = self.model.row_count();
         let start_x = layout_node.layout.location.x;
         let start_y = layout_node.layout.location.y;
         let width = layout_node.layout.size.width;
         
         let item_width = 100.0; // TODO: Configurable
         let item_height = 100.0;
         
         let cols = (width / item_width).floor() as usize;
         let cols = cols.max(1);
         
         let palette = context.palette();
         
         for i in 0..rows {
             let is_selected = self.selected_rows.get().contains(&i);
             
             let row = i / cols;
             let col = i % cols;
             
             let x = start_x + (col as f32 * item_width);
             let y = start_y + (row as f32 * item_height);
             
             // Culling
             if y > start_y + layout_node.layout.size.height {
                 break;
             }
             if y + item_height < start_y {
                 continue;
             }
             
             let item_rect = Rect::new(
                 x as f64,
                 y as f64,
                 (x + item_width) as f64,
                 (y + item_height) as f64
             );

            // Selection background
            if is_selected {
                let selection_color = palette.color(ColorRole::Selection);
                let rounded = nptk_core::vg::kurbo::RoundedRect::new(item_rect.x0, item_rect.y0, item_rect.x1, item_rect.y1, 4.0);
                graphics.fill(
                     nptk_core::vg::peniko::Fill::NonZero,
                     Affine::IDENTITY,
                     &Brush::Solid(selection_color),
                     None,
                     &Self::shape_to_path(&rounded)
                );
            }

             // Icon
             if let ModelData::String(icon_name) = self.model.data(i, 0, ItemRole::Icon) {
                 // Draw Icon centered
                 let icon_size = 48.0;
                 let icon_x = x + (item_width - icon_size) / 2.0;
                 let icon_y = y + (item_height - icon_size) / 2.0 - 10.0; // Slightly up to leave room for text
                 
                 let icon_rect = Rect::new(
                     icon_x as f64,
                     icon_y as f64,
                     (icon_x + icon_size) as f64,
                     (icon_y + icon_size) as f64
                 );
                 self.draw_icon(graphics, &icon_name, icon_rect, &palette);
             }
             
             // Text
             if let ModelData::String(text) = self.model.data(i, 0, ItemRole::Display) {
                 let text_brush = Brush::Solid(palette.color(ColorRole::WindowText));
                 self.text_context.render_text(
                     &mut info.font_context,
                     graphics,
                     &text,
                     None,
                     12.0,
                     text_brush,
                     Affine::translate((x as f64 + 5.0, y as f64 + 70.0)), // Text at bottom
                     true,
                     Some((item_width - 10.0) as f32)
                 );
             }
         }
    }

    // Helper to avoid duplicate shape_to_path logic if not available elsewhere
    fn shape_to_path(shape: &impl Shape) -> nptk_core::vg::peniko::kurbo::BezPath {
        shape.path_elements(0.1).collect()
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
                        Some(col_width as f32 - 30.0), // Leave space for sort arrow
                    );
                 
                 // Draw sort indicator if this column is sorted
                 if let Some((sorted_col, sort_order)) = self.sorted_column.get().as_ref() {
                     if *sorted_col == c {
                         let arrow_x = x + col_width - 15.0;
                         let arrow_y = layout_node.layout.location.y as f64 + 15.0;
                         let arrow_size = 4.0;
                         
                         // Draw triangle based on sort order
                         let (p1, p2, p3) = match sort_order {
                             SortOrder::Ascending => {
                                 // Upward triangle
                                 (
                                     nptk_core::vg::kurbo::Point::new(arrow_x, arrow_y - arrow_size),
                                     nptk_core::vg::kurbo::Point::new(arrow_x - arrow_size, arrow_y + arrow_size),
                                     nptk_core::vg::kurbo::Point::new(arrow_x + arrow_size, arrow_y + arrow_size),
                                 )
                             }
                             SortOrder::Descending => {
                                 // Downward triangle
                                 (
                                     nptk_core::vg::kurbo::Point::new(arrow_x, arrow_y + arrow_size),
                                     nptk_core::vg::kurbo::Point::new(arrow_x - arrow_size, arrow_y - arrow_size),
                                     nptk_core::vg::kurbo::Point::new(arrow_x + arrow_size, arrow_y - arrow_size),
                                 )
                             }
                         };
                         
                         // Create triangle path
                         let mut path = nptk_core::vg::kurbo::BezPath::new();
                         path.move_to(p1);
                         path.line_to(p2);
                         path.line_to(p3);
                         path.close_path();
                         
                         graphics.fill(
                             nptk_core::vg::peniko::Fill::NonZero,
                             Affine::IDENTITY,
                             &Brush::Solid(text_color),
                             None,
                             &path,
                         );
                     }
                 }
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
                 let mut text_offset = 5.0;
                 let icon_size = 16.0;

                 // Draw Icon
                 if let ModelData::String(icon_name) = self.model.data(i, c, ItemRole::Icon) {
                     let icon_rect = Rect::new(
                         x + 5.0,
                         y as f64 + (self.item_height as f64 - icon_size) / 2.0,
                         x + 5.0 + icon_size,
                         y as f64 + (self.item_height as f64 + icon_size) / 2.0
                     );
                     self.draw_icon(graphics, &icon_name, icon_rect, &palette);
                     text_offset += icon_size + 5.0;
                 }
                
                let text_data = self.model.data(i, c, ItemRole::Display);
                if let ModelData::String(text) = text_data {
                    let text_color = palette.color(ColorRole::BaseText);
                    let transform = Affine::translate((cell_rect.x0 + text_offset, cell_rect.y0 + 5.0));
                    
                     self.text_context.render_text(
                        &mut info.font_context,
                        graphics,
                        &text,
                        None,
                        16.0,
                        Brush::Solid(text_color),
                        transform,
                        true,
                        Some(cell_rect.width() as f32 - text_offset as f32 - 5.0),
                    );
                }
            }
            
            y += self.item_height;
         }
    }


    fn draw_icon(&self, graphics: &mut dyn Graphics, name: &str, rect: Rect, palette: &Palette) {
        use nptk_core::vg::kurbo::{BezPath, Point, Stroke};
        
        let color = if name == "directory" {
             // Folder color (yellow-ish/orange)
             nptk_core::vg::peniko::Color::from_rgb8(240, 180, 60)
        } else {
             // File color (white/gray)
             palette.color(ColorRole::BaseText).with_alpha(0.7)
        };
        
        let icon_path = if name == "directory" {
             // Draw folder shape
             let mut path = BezPath::new();
             let w = rect.width();
             let h = rect.height();
             let x = rect.x0;
             let y = rect.y0;
             
             path.move_to(Point::new(x, y + h * 0.15));
             path.line_to(Point::new(x + w * 0.4, y + h * 0.15));
             path.line_to(Point::new(x + w * 0.5, y));
             path.line_to(Point::new(x + w, y));
             path.line_to(Point::new(x + w, y + h));
             path.line_to(Point::new(x, y + h));
             path.close_path();
             path
        } else {
             // Draw file shape
             let mut path = BezPath::new();
             let w = rect.width() * 0.8; // Make file slighty narrower
             let h = rect.height();
             let x = rect.x0 + (rect.width() - w) / 2.0;
             let y = rect.y0;
             
             path.move_to(Point::new(x, y));
             path.line_to(Point::new(x + w * 0.7, y));
             path.line_to(Point::new(x + w, y + h * 0.25));
             path.line_to(Point::new(x + w, y + h));
             path.line_to(Point::new(x, y + h));
             path.close_path();
             // Fold corner
             path.move_to(Point::new(x + w * 0.7, y));
             path.line_to(Point::new(x + w * 0.7, y + h * 0.25));
             path.line_to(Point::new(x + w, y + h * 0.25));
             path
        };
        
        graphics.fill(
             nptk_core::vg::peniko::Fill::NonZero,
             Affine::IDENTITY,
             &Brush::Solid(color),
             None,
             &icon_path
        );
        
        // Stroke
        graphics.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(palette.color(ColorRole::WindowText).with_alpha(0.3)),
            None,
            &icon_path
        );
    }

    fn render_compact(&mut self, graphics: &mut dyn Graphics, layout_node: &LayoutNode, info: &mut AppInfo, context: &AppContext) {
         // Re-use list rendering for now
         self.render_list(graphics, layout_node, info, context);
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

    async fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        
        let _mode = *self.view_mode.get(); // Kept for logic if needed, but unused error suggests removing or prefixing
        // Actually, just prefix it
        let _mode = *self.view_mode.get();

        
        // Handle Keyboard Events for Navigation
        for (_, key_event) in &info.keys {
            if key_event.state == ElementState::Pressed && !key_event.repeat {
                use nptk_core::window::Key;
                
                match &key_event.logical_key {
                    Key::Named(named_key) => {
                        use nptk_core::window::NamedKey;
                        
                        let current_selection = self.selected_rows.get().clone();
                        let row_count = self.model.row_count();
                        
                        match named_key {
                            NamedKey::ArrowDown => {
                                // Move selection down
                                if let Some(&last_selected) = current_selection.last() {
                                    if last_selected + 1 < row_count {
                                        let new_index = last_selected + 1;
                                        let mut new_selection = if info.modifiers.shift_key() {
                                            // Extend selection
                                            let mut sel = current_selection.clone();
                                            sel.push(new_index);
                                            sel
                                        } else {
                                            vec![new_index]
                                        };
                                        new_selection.sort_unstable();
                                        new_selection.dedup();
                                        
                                        if let Some(signal) = self.selected_rows.as_signal() {
                                            signal.set(new_selection.clone());
                                        }
                                        if let Some(cb) = &self.on_selection_change {
                                            update |= cb(new_selection);
                                        }
                                        self.last_selected_index = Some(new_index);
                                        update.insert(Update::DRAW);
                                    }
                                } else if row_count > 0 {
                                    // No selection, select first item
                                    if let Some(signal) = self.selected_rows.as_signal() {
                                        signal.set(vec![0]);
                                    }
                                    if let Some(cb) = &self.on_selection_change {
                                        update |= cb(vec![0]);
                                    }
                                    self.last_selected_index = Some(0);
                                    update.insert(Update::DRAW);
                                }
                            }
                            NamedKey::ArrowUp => {
                                // Move selection up
                                if let Some(&first_selected) = current_selection.first() {
                                    if first_selected > 0 {
                                        let new_index = first_selected - 1;
                                        let mut new_selection = if info.modifiers.shift_key() {
                                            // Extend selection
                                            let mut sel = current_selection.clone();
                                            sel.push(new_index);
                                            sel
                                        } else {
                                            vec![new_index]
                                        };
                                        new_selection.sort_unstable();
                                        new_selection.dedup();
                                        
                                        if let Some(signal) = self.selected_rows.as_signal() {
                                            signal.set(new_selection.clone());
                                        }
                                        if let Some(cb) = &self.on_selection_change {
                                            update |= cb(new_selection);
                                        }
                                        self.last_selected_index = Some(new_index);
                                        update.insert(Update::DRAW);
                                    }
                                } else if row_count > 0 {
                                    // No selection, select last item
                                    let last_idx = row_count - 1;
                                    if let Some(signal) = self.selected_rows.as_signal() {
                                        signal.set(vec![last_idx]);
                                    }
                                    if let Some(cb) = &self.on_selection_change {
                                        update |= cb(vec![last_idx]);
                                    }
                                    self.last_selected_index = Some(last_idx);
                                    update.insert(Update::DRAW);
                                }
                            }
                            NamedKey::Enter => {
                                // Activate selected item
                                if let Some(&selected) = current_selection.first() {
                                    if let Some(cb) = &self.on_activate {
                                        update |= cb(selected);
                                    }
                                }
                            }
                            NamedKey::Space => {
                                // Toggle selection (like Ctrl+click)
                                if let Some(last_idx) = self.last_selected_index {
                                    let mut new_selection = current_selection.clone();
                                    if let Some(pos) = new_selection.iter().position(|&r| r == last_idx) {
                                        new_selection.remove(pos);
                                    } else {
                                        new_selection.push(last_idx);
                                    }
                                    new_selection.sort_unstable();
                                    new_selection.dedup();
                                    
                                    if let Some(signal) = self.selected_rows.as_signal() {
                                        signal.set(new_selection.clone());
                                    }
                                    if let Some(cb) = &self.on_selection_change {
                                        update |= cb(new_selection);
                                    }
                                    update.insert(Update::DRAW);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Handle Mouse Events for Selection
        if let Some(pos) = info.cursor_pos {
            let rect = layout.layout;
            if pos.x >= rect.location.x as f64 && pos.x <= (rect.location.x + rect.size.width) as f64 &&
               pos.y >= rect.location.y as f64 && pos.y <= (rect.location.y + rect.size.height) as f64 {
                // Check for clicks
                let left_click_current = info.buttons.iter().any(|(_, btn, state)| *btn == MouseButton::Left && *state == ElementState::Pressed);
                
                // Only trigger if pressed NOW and NOT pressed BEFORE (Edge detection)
                let left_click = left_click_current && !self.was_left_down;
                
                // Update state for next frame
                self.was_left_down = left_click_current;

                if left_click {
                    // Check for header click in Table mode
                    if *self.view_mode.get() == ViewMode::Table {
                        let local_y = pos.y - (layout.layout.location.y as f64);
                        if local_y < 30.0 {
                            // Header click
                            let cols = self.model.column_count();
                            let col_width = (layout.layout.size.width / cols as f32) as f64;
                            let local_x = pos.x - (layout.layout.location.x as f64);
                            
                            let clicked_col = (local_x / col_width).floor() as usize;
                            if clicked_col < cols {
                                // Toggle sort
                                let new_order = if let Some((current_col, current_order)) = self.sorted_column.get().as_ref() {
                                    if *current_col == clicked_col {
                                        match current_order {
                                            SortOrder::Ascending => SortOrder::Descending,
                                            SortOrder::Descending => SortOrder::Ascending,
                                        }
                                    } else {
                                        SortOrder::Ascending
                                    }
                                } else {
                                    SortOrder::Ascending
                                };
                                
                                // Update model (generic model needs to support sorting)
                                self.model.sort(clicked_col, new_order);
                                
                                // Update state
                                if let Some(signal) = self.sorted_column.as_signal() {
                                    signal.set(Some((clicked_col, new_order)));
                                }
                                
                                update.insert(Update::DRAW);
                                return update;
                            }
                        }
                    }

                    // Calculate clicked row
                     let item_y = if *self.view_mode.get() == ViewMode::Table {
                        pos.y - (layout.layout.location.y as f64) - 30.0 // Minus header
                    } else {
                        pos.y - (layout.layout.location.y as f64)
                    };
                    
                    let row_index = if *self.view_mode.get() == ViewMode::Icon {
                         let item_width = 100.0;
                         let item_height = 100.0;
                         let width = layout.layout.size.width as f64;
                         let cols = (width / item_width).floor() as usize;
                         let cols = cols.max(1);
                         
                         let local_x = pos.x - (layout.layout.location.x as f64);
                         let local_y = pos.y - (layout.layout.location.y as f64);
                         
                         if local_x >= 0.0 && local_y >= 0.0 {
                             let row = (local_y / item_height).floor() as usize;
                             let col = (local_x / item_width).floor() as usize;
                             if col < cols {
                                 Some(row * cols + col)
                             } else {
                                 None
                             }
                         } else {
                             None
                         }
                    } else if item_y >= 0.0 {
                        Some((item_y / self.item_height as f64).floor() as usize)
                    } else {
                        None
                    };
                     
                     if let Some(row_index) = row_index {
                         if row_index < self.model.row_count() {
                             let mut current_selection = self.selected_rows.get().clone();
                             
                            // Multi-selection support
                            if info.modifiers.control_key() {
                                // Ctrl: Toggle selection
                                if let Some(pos) = current_selection.iter().position(|&r| r == row_index) {
                                    current_selection.remove(pos);
                                } else {
                                    current_selection.push(row_index);
                                }
                                self.last_selected_index = Some(row_index);
                            } else if info.modifiers.shift_key() {
                                // Shift: Range selection
                                if let Some(last_idx) = self.last_selected_index {
                                    let start = last_idx.min(row_index);
                                    let end = last_idx.max(row_index);
                                    current_selection.clear();
                                    for idx in start..=end {
                                        if idx < self.model.row_count() {
                                            current_selection.push(idx);
                                        }
                                    }
                                } else {
                                    current_selection.clear();
                                    current_selection.push(row_index);
                                    self.last_selected_index = Some(row_index);
                                }
                            } else {
                                // Normal click: Single selection
                                current_selection.clear();
                                current_selection.push(row_index);
                                self.last_selected_index = Some(row_index);
                            }
                            
                            // Sort selection for consistency
                            current_selection.sort_unstable();
                            current_selection.dedup();
                            
                            if let Some(signal) = self.selected_rows.as_signal() {
                                signal.set(current_selection.clone());
                            }
                            
                            if let Some(cb) = &self.on_selection_change {
                                update |= cb(current_selection);
                            }
                            
                            // Check for activation (double click)
                            let now = Instant::now();
                            if let Some((last_row, last_time)) = self.last_click {
                                if last_row == row_index && now.duration_since(last_time) < Duration::from_millis(500) {
                                    if let Some(cb) = &self.on_activate {
                                         update |= cb(row_index);
                                    }
                                    self.last_click = None; // Reset
                                } else {
                                    self.last_click = Some((row_index, now));
                                }
                            } else {
                                self.last_click = Some((row_index, now));
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
        let mode = *self.view_mode.get();
        // eprintln!("ItemView::render: mode={:?}", mode);
        
        match mode {
            ViewMode::List => self.render_list(graphics, layout, info, &context),
            ViewMode::Icon => self.render_icon(graphics, layout, info, &context),
            ViewMode::Compact => self.render_compact(graphics, layout, info, &context),
            ViewMode::Table => self.render_table(graphics, layout, info, &context),
        }
    }
}

impl WidgetLayoutExt for ItemView {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}


