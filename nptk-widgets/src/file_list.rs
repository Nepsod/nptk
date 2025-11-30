use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{Affine, Rect, Shape};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_services::filesystem::entry::{FileEntry, FileType};
use nptk_services::filesystem::model::{FileSystemEvent, FileSystemModel};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use tokio::sync::broadcast;

use crate::scroll_container::{ScrollContainer, ScrollDirection};

/// A widget that displays a list of files.
pub struct FileList {
    // State
    current_path: StateSignal<PathBuf>,
    entries: StateSignal<Vec<FileEntry>>,
    selected_path: StateSignal<Option<PathBuf>>,
    
    // Model
    fs_model: Arc<FileSystemModel>,
    _event_rx: Arc<Mutex<broadcast::Receiver<FileSystemEvent>>>,
    
    // Layout
    layout_style: MaybeSignal<LayoutStyle>,
    
    // Child widgets
    scroll_container: BoxedWidget,
    
    // Track if signals are hooked
    signals_hooked: bool,
}

impl FileList {
    /// Create a new file list widget.
    pub fn new(initial_path: PathBuf) -> Self {
        let fs_model = Arc::new(FileSystemModel::new(initial_path.clone()).unwrap());
        let event_rx = Arc::new(Mutex::new(fs_model.subscribe_events()));
        
        // Initial load
        let _ = fs_model.refresh(&initial_path);
        
        let current_path = StateSignal::new(initial_path.clone());
        let entries = StateSignal::new(Vec::new());
        let selected_path = StateSignal::new(None);
        
        // Create content widget
        let content = FileListContent::new(
            entries.clone(),
            selected_path.clone(),
            current_path.clone(),
            fs_model.clone(),
        );
        
        // Create scroll container
        let scroll_container = ScrollContainer::new()
            .with_scroll_direction(ScrollDirection::Vertical)
            .with_virtual_scrolling(true, 30.0)
            .with_child(content);
            
        Self {
            current_path,
            entries,
            selected_path,
            fs_model,
            _event_rx: event_rx,
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            }.into(),
            scroll_container: Box::new(scroll_container),
            signals_hooked: false,
        }
    }
    
    /// Set the current path.
    pub fn set_path(&mut self, path: PathBuf) {
        self.current_path.set(path.clone());
        // Trigger reload in model
        let _ = self.fs_model.refresh(&path);
    }
    
    /// Get the currently selected path.
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_path.get().clone()
    }
}

impl Widget for FileList {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "FileList")
    }
    
    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![self.scroll_container.layout_style()],
        }
    }
    
    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        // Hook signals on first update to make them reactive
        if !self.signals_hooked {
            context.hook_signal(&mut self.entries);
            context.hook_signal(&mut self.current_path);
            context.hook_signal(&mut self.selected_path);
            self.signals_hooked = true;
        }
        
        let mut update = Update::empty();
        
        // Poll filesystem events
        if let Ok(mut rx) = self._event_rx.try_lock() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    FileSystemEvent::DirectoryLoaded { path, entries } => {
                        if path == *self.current_path.get() {
                            self.entries.set(entries);
                            update.insert(Update::LAYOUT | Update::DRAW);
                        }
                    }
                    _ => {
                        // For other events, we might want to refresh if they affect current path
                        // But for now, let's just rely on DirectoryLoaded
                    }
                }
            }
        }
        
        // Update child (ScrollContainer)
        if !layout.children.is_empty() {
             update |= self.scroll_container.update(&layout.children[0], context.clone(), info);
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
        // Render ScrollContainer
        if !layout.children.is_empty() {
            self.scroll_container.render(graphics, theme, &layout.children[0], info, context);
        }
    }
}

impl WidgetLayoutExt for FileList {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}

/// Inner widget that renders the actual list content.
struct FileListContent {
    entries: StateSignal<Vec<FileEntry>>,
    selected_path: StateSignal<Option<PathBuf>>,
    current_path: StateSignal<PathBuf>,
    fs_model: Arc<FileSystemModel>,
    
    item_height: f32,
    text_render_context: TextRenderContext,
    
    // Input state
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
}

impl FileListContent {
    fn new(
        entries: StateSignal<Vec<FileEntry>>,
        selected_path: StateSignal<Option<PathBuf>>,
        current_path: StateSignal<PathBuf>,
        fs_model: Arc<FileSystemModel>,
    ) -> Self {
        Self {
            entries,
            selected_path,
            current_path,
            fs_model,
            item_height: 30.0,
            text_render_context: TextRenderContext::new(),
            last_click_time: None,
            last_click_index: None,
        }
    }
}

impl Widget for FileListContent {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "FileListContent")
    }
    
    fn layout_style(&self) -> StyleNode {
        let count = self.entries.get().len();
        let height = (count as f32 * self.item_height).max(100.0);
        
        StyleNode {
            style: LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::length(height)),
                ..Default::default()
            },
            children: vec![],
        }
    }
    
    fn update(&mut self, layout: &LayoutNode, _context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        
        if let Some(cursor) = info.cursor_pos {
            // Check for clicks
            let local_y = cursor.y as f32 - layout.layout.location.y;
            let local_x = cursor.x as f32 - layout.layout.location.x;
            
            // Check bounds
            if local_x >= 0.0 && local_x < layout.layout.size.width &&
               local_y >= 0.0 && local_y < layout.layout.size.height 
            {
                let index = (local_y / self.item_height) as usize;
                let entries = self.entries.get();
                
                if index < entries.len() {
                    let entry = &entries[index];
                    
                    for (_, btn, el) in &info.buttons {
                        if *btn == MouseButton::Left && *el == ElementState::Pressed {
                            // Clicked
                            self.selected_path.set(Some(entry.path.clone()));
                            update.insert(Update::DRAW);
                            
                            // Check double click
                            let now = Instant::now();
                            if let Some(last_time) = self.last_click_time {
                                if let Some(last_index) = self.last_click_index {
                                    if last_index == index && now.duration_since(last_time) < Duration::from_millis(500) {
                                        // Double click
                                        if entry.file_type == FileType::Directory {
                                            // Navigate
                                            self.current_path.set(entry.path.clone());
                                            let _ = self.fs_model.refresh(&entry.path);
                                            self.selected_path.set(None);
                                            update.insert(Update::LAYOUT);
                                        }
                                    }
                                }
                            }
                            
                            self.last_click_time = Some(now);
                            self.last_click_index = Some(index);
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
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        let entries = self.entries.get();
        let selected = self.selected_path.get();
        let entry_count = entries.len();
        
        // Draw background to verify rendering is working
        let bg_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );
        
        let bg_color = theme
            .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackground)
            .unwrap_or(Color::from_rgb8(255, 255, 255));
        
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg_color),
            None,
            &bg_rect.to_path(0.1),
        );
        
        if entry_count == 0 {
            return;
        }
        
        // We should only render visible items if we had viewport info, but here we render all
        // relying on clipping in parent.
        // However, for performance, we should probably check what's visible.
        // But `layout` here is the full size of the list.
        // The parent `ScrollContainer` clips, but we still issue draw commands for everything.
        // Ideally `ScrollContainer` would pass visible range info or we'd calculate it.
        // But for now, let's render all (or optimize later).
        
        for (i, entry) in entries.iter().enumerate() {
            let y = layout.layout.location.y + i as f32 * self.item_height;
            let row_rect = Rect::new(
                layout.layout.location.x as f64,
                y as f64,
                (layout.layout.location.x + layout.layout.size.width) as f64,
                (y + self.item_height) as f64,
            );
            
            // Draw selection background
            if Some(&entry.path) == selected.as_ref() {
                let color = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                    .unwrap_or(Color::from_rgb8(0, 120, 215)); // Default blue
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(0.3)),
                    None,
                    &row_rect.to_path(0.1),
                );
            }
            
            // Draw icon (placeholder)
            let icon_color = if entry.file_type == FileType::Directory {
                Color::from_rgb8(255, 200, 100) // Orange for folders
            } else {
                Color::from_rgb8(200, 200, 200) // Gray for files
            };
            
            let icon_rect = Rect::new(
                row_rect.x0 + 5.0,
                row_rect.y0 + 5.0,
                row_rect.x0 + 25.0,
                row_rect.y1 - 5.0,
            );
            
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(icon_color),
                None,
                &icon_rect.to_path(0.1),
            );
            
            // Draw text
            let text_color = theme.get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                .unwrap_or(Color::BLACK);
                
            let transform = Affine::translate((
                row_rect.x0 + 35.0,
                row_rect.y0 + 5.0, // Vertical alignment adjustment
            ));
            
            self.text_render_context.render_text(
                &mut info.font_context,
                graphics,
                &entry.name,
                None,
                16.0,
                Brush::Solid(text_color),
                transform,
                true,
                Some(row_rect.width() as f32 - 40.0),
            );
        }
    }
}
