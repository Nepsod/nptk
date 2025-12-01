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
use nptk_core::vg::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Shape, Vec2};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_services::filesystem::entry::{FileEntry, FileType};
use nptk_services::filesystem::model::{FileSystemEvent, FileSystemModel};
use nptk_services::icon::IconRegistry;
use nptk_services::thumbnail::{ThumbnailProvider, ThumbnailifyProvider};
use nptk_services::thumbnail::events::ThumbnailEvent;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use tokio::sync::broadcast;
use std::collections::HashSet;

use crate::scroll_container::{ScrollContainer, ScrollDirection};
use nptk_services::thumbnail::ThumbnailImageCache;

/// View mode for the file list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileListViewMode {
    /// List view (icon + text in rows)
    List,
    /// Large icon view (grid layout with icons and labels below)
    Icon,
}

/// A widget that displays a list of files.
pub struct FileList {
    // State
    current_path: StateSignal<PathBuf>,
    entries: StateSignal<Vec<FileEntry>>,
    selected_paths: StateSignal<Vec<PathBuf>>,
    view_mode: StateSignal<FileListViewMode>,
    icon_size: StateSignal<u32>,
    
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
        let selected_paths = StateSignal::new(Vec::new());
        let view_mode = StateSignal::new(FileListViewMode::List);
        let icon_size = StateSignal::new(48);
        
        // Create icon registry
        let icon_registry = Arc::new(
            IconRegistry::new().unwrap_or_else(|_| IconRegistry::default())
        );
        
        // Create thumbnail provider
        let provider = ThumbnailifyProvider::new();
        let thumbnail_event_rx = provider.subscribe();
        let thumbnail_provider: Arc<dyn ThumbnailProvider> = Arc::new(provider);
        
        // Create content widget
        let content = FileListContent::new(
            entries.clone(),
            selected_paths.clone(),
            current_path.clone(),
            view_mode.clone(),
            icon_size.clone(),
            fs_model.clone(),
            icon_registry.clone(),
            thumbnail_provider.clone(),
            thumbnail_event_rx,
        );
        
        // Create scroll container (Both directions to support icon view)
        let scroll_container = ScrollContainer::new()
            .with_scroll_direction(ScrollDirection::Both)
            .with_virtual_scrolling(true, 30.0)
            .with_child(content);
            
        Self {
            current_path,
            entries,
            selected_paths,
            view_mode,
            icon_size,
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
    
    /// Get the currently selected paths.
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected_paths.get().clone()
    }
    
    /// Get the first selected path (for backward compatibility).
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_paths.get().first().cloned()
    }
    
    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selected_paths.set(Vec::new());
    }
    
    /// Select all entries.
    pub fn select_all(&mut self) {
        let entries = self.entries.get();
        let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
        self.selected_paths.set(paths);
    }
    
    /// Set the view mode.
    pub fn set_view_mode(&mut self, mode: FileListViewMode) {
        self.view_mode.set(mode);
    }
    
    /// Set the icon size for icon view.
    pub fn set_icon_size(&mut self, size: u32) {
        self.icon_size.set(size);
    }
    
    /// Set the view mode (builder pattern).
    pub fn with_view_mode(self, mode: FileListViewMode) -> Self {
        let mut new_self = self;
        new_self.view_mode.set(mode);
        new_self
    }
    
    /// Set the icon size (builder pattern).
    pub fn with_icon_size(self, size: u32) -> Self {
        let mut new_self = self;
        new_self.icon_size.set(size);
        new_self
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
            context.hook_signal(&mut self.selected_paths);
            context.hook_signal(&mut self.view_mode);
            context.hook_signal(&mut self.icon_size);
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
    selected_paths: StateSignal<Vec<PathBuf>>,
    current_path: StateSignal<PathBuf>,
    view_mode: StateSignal<FileListViewMode>,
    icon_size: StateSignal<u32>,
    fs_model: Arc<FileSystemModel>,
    icon_registry: Arc<IconRegistry>,
    thumbnail_provider: Arc<dyn ThumbnailProvider>,
    
    item_height: f32,
    text_render_context: TextRenderContext,
    thumbnail_size: u32,
    
    // Input state
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    anchor_index: Option<usize>, // For Shift+Click range selection
    
    // Icon cache per entry (to avoid repeated lookups)
    icon_cache: Arc<Mutex<std::collections::HashMap<(PathBuf, u32), Option<nptk_services::icon::CachedIcon>>>>,
    
    // Thumbnail cache for decoded images
    thumbnail_cache: Arc<ThumbnailImageCache>,
    
    // Track pending thumbnail requests to avoid duplicate requests
    pending_thumbnails: Arc<Mutex<HashSet<PathBuf>>>,
    
    // Thumbnail event receiver
    thumbnail_event_rx: Arc<Mutex<tokio::sync::broadcast::Receiver<ThumbnailEvent>>>,
    
    // Icon view constants
    icon_view_padding: f32,
    icon_view_spacing: f32,
    icon_view_text_height: f32,
}

impl FileListContent {
    fn new(
        entries: StateSignal<Vec<FileEntry>>,
        selected_paths: StateSignal<Vec<PathBuf>>,
        current_path: StateSignal<PathBuf>,
        view_mode: StateSignal<FileListViewMode>,
        icon_size: StateSignal<u32>,
        fs_model: Arc<FileSystemModel>,
        icon_registry: Arc<IconRegistry>,
        thumbnail_provider: Arc<dyn ThumbnailProvider>,
        thumbnail_event_rx: tokio::sync::broadcast::Receiver<ThumbnailEvent>,
    ) -> Self {
        Self {
            entries,
            selected_paths,
            current_path,
            view_mode,
            icon_size,
            fs_model,
            icon_registry,
            thumbnail_provider,
            item_height: 30.0,
            text_render_context: TextRenderContext::new(),
            thumbnail_size: 128, // Default thumbnail size
            last_click_time: None,
            last_click_index: None,
            anchor_index: None,
            icon_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            thumbnail_cache: Arc::new(ThumbnailImageCache::default()),
            pending_thumbnails: Arc::new(Mutex::new(HashSet::new())),
            thumbnail_event_rx: Arc::new(Mutex::new(thumbnail_event_rx)),
            icon_view_padding: 10.0,
            icon_view_spacing: 8.0,
            icon_view_text_height: 40.0,
        }
    }
    
    /// Set the thumbnail size for this file list.
    pub fn with_thumbnail_size(mut self, size: u32) -> Self {
        self.thumbnail_size = size;
        self
    }
    
    /// Check if a path is selected.
    fn is_selected(&self, path: &PathBuf) -> bool {
        self.selected_paths.get().contains(path)
    }
    
    /// Calculate icon view layout parameters.
    fn calculate_icon_view_layout(&self, viewport_width: f32, icon_size: u32) -> (usize, f32, f32) {
        let icon_size_f = icon_size as f32;
        let cell_width = icon_size_f + self.icon_view_spacing * 2.0;
        let available_width = viewport_width - self.icon_view_padding * 2.0;
        let columns = (available_width / cell_width).floor().max(1.0) as usize;
        let cell_height = icon_size_f + self.icon_view_text_height + self.icon_view_spacing * 2.0;
        (columns, cell_width, cell_height)
    }
    
    /// Get icon position in grid layout.
    fn get_icon_position(&self, index: usize, columns: usize, cell_width: f32, cell_height: f32) -> (f32, f32) {
        let col = index % columns;
        let row = index / columns;
        let x = self.icon_view_padding + col as f32 * cell_width;
        let y = self.icon_view_padding + row as f32 * cell_height;
        (x, y)
    }
    
    fn render_list_view(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
    ) {
        let entries = self.entries.get();
        let selected_paths = self.selected_paths.get();
        let selected_set: HashSet<&PathBuf> = selected_paths.iter().collect();
        let entry_count = entries.len();
        
        // Draw background
        let bg_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );
        
        let bg_color = theme
            .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackground)
            .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackground))
            .unwrap_or_else(|| theme.window_background());
        
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
        
        for (i, entry) in entries.iter().enumerate() {
            let y = layout.layout.location.y + i as f32 * self.item_height;
            let row_rect = Rect::new(
                layout.layout.location.x as f64,
                y as f64,
                (layout.layout.location.x + layout.layout.size.width) as f64,
                (y + self.item_height) as f64,
            );
            
            // Check for hover state
            let is_hovered = if let Some(cursor) = info.cursor_pos {
                let cursor_x = cursor.x as f64;
                let cursor_y = cursor.y as f64;
                cursor_x >= row_rect.x0 && cursor_x < row_rect.x1 &&
                cursor_y >= row_rect.y0 && cursor_y < row_rect.y1
            } else {
                false
            };
            
            // Draw hover background (if not selected)
            if is_hovered && !selected_set.contains(&entry.path) {
                let hover_color = theme
                    .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorMenuHovered)
                    .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorMenuHovered))
                    .unwrap_or_else(|| Color::from_rgb8(240, 240, 240));
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(hover_color),
                    None,
                    &row_rect.to_path(0.1),
                );
            }
            
            // Draw selection background
            if selected_set.contains(&entry.path) {
                let color = theme
                    .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                    .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackgroundSelected))
                    .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(0.3)),
                    None,
                    &row_rect.to_path(0.1),
                );
            }
            
            // Try to get thumbnail first, fall back to icon
            let icon_size = 20.0;
            let icon_rect = Rect::new(
                row_rect.x0 + 5.0,
                row_rect.y0 + 5.0,
                row_rect.x0 + 25.0,
                row_rect.y1 - 5.0,
            );
            
            // Check if thumbnail is available
            let mut use_thumbnail = false;
            if let Some(thumbnail_path) = self.thumbnail_provider.get_thumbnail(entry, self.thumbnail_size) {
                if let Ok(Some(cached_thumb)) = self.thumbnail_cache.load_or_get(&thumbnail_path, self.thumbnail_size) {
                    use nptk_core::vg::peniko::{Blob, ImageBrush, ImageData, ImageFormat, ImageAlphaType};
                    let image_data = ImageData {
                        data: Blob::from(cached_thumb.data.as_ref().clone()),
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width: cached_thumb.width,
                        height: cached_thumb.height,
                    };
                    let image_brush = ImageBrush::new(image_data);
                    let icon_x = icon_rect.x0;
                    let icon_y = icon_rect.y0;
                    let icon_size_f64 = icon_rect.width().min(icon_rect.height());
                    let scale_x = icon_size_f64 / (cached_thumb.width as f64);
                    let scale_y = icon_size_f64 / (cached_thumb.height as f64);
                    let scale = scale_x.min(scale_y);
                    let transform = Affine::scale_non_uniform(scale, scale)
                        .then_translate(Vec2::new(icon_x, icon_y));
                    if let Some(scene) = graphics.as_scene_mut() {
                        scene.draw_image(&image_brush, transform);
                    }
                    use_thumbnail = true;
                }
            }
            
            // If no thumbnail, use icon
            if !use_thumbnail {
                if self.thumbnail_provider.is_supported(entry) {
                    let mut pending = self.pending_thumbnails.lock().unwrap();
                    if !pending.contains(&entry.path) {
                        if let Ok(()) = self.thumbnail_provider.request_thumbnail(entry, self.thumbnail_size) {
                            pending.insert(entry.path.clone());
                        }
                    }
                }
                
                let cache_key = (entry.path.clone(), icon_size as u32);
                let cached_icon = {
                    let mut cache = self.icon_cache.lock().unwrap();
                    if let Some(icon) = cache.get(&cache_key) {
                        icon.clone()
                    } else {
                        let icon = self.icon_registry.get_file_icon(entry, icon_size as u32);
                        cache.insert(cache_key.clone(), icon.clone());
                        icon
                    }
                };
                
                if let Some(icon) = cached_icon {
                    let icon_x = icon_rect.x0;
                    let icon_y = icon_rect.y0;
                    let icon_size_f64 = icon_rect.width().min(icon_rect.height());
                    
                    match icon {
                        nptk_services::icon::CachedIcon::Image { data, width, height } => {
                            use nptk_core::vg::peniko::{Blob, ImageBrush, ImageData, ImageFormat, ImageAlphaType};
                            let image_data = ImageData {
                                data: Blob::from(data.as_ref().clone()),
                                format: ImageFormat::Rgba8,
                                alpha_type: ImageAlphaType::Alpha,
                                width,
                                height,
                            };
                            let image_brush = ImageBrush::new(image_data);
                            let scale_x = icon_size_f64 / (width as f64);
                            let scale_y = icon_size_f64 / (height as f64);
                            let scale = scale_x.min(scale_y);
                            let transform = Affine::scale_non_uniform(scale, scale)
                                .then_translate(Vec2::new(icon_x, icon_y));
                            if let Some(scene) = graphics.as_scene_mut() {
                                scene.draw_image(&image_brush, transform);
                            }
                        }
                        nptk_services::icon::CachedIcon::Svg(svg_source) => {
                            use vello_svg::usvg::{Tree, Options, ShapeRendering, TextRendering, ImageRendering};
                            if let Ok(tree) = Tree::from_str(
                                svg_source.as_str(),
                                &Options {
                                    shape_rendering: ShapeRendering::GeometricPrecision,
                                    text_rendering: TextRendering::OptimizeLegibility,
                                    image_rendering: ImageRendering::OptimizeSpeed,
                                    ..Default::default()
                                },
                            ) {
                                let scene = vello_svg::render_tree(&tree);
                                let svg_size = tree.size();
                                let scale_x = icon_size_f64 / svg_size.width() as f64;
                                let scale_y = icon_size_f64 / svg_size.height() as f64;
                                let scale = scale_x.min(scale_y);
                                let transform = Affine::scale_non_uniform(scale, scale)
                                    .then_translate(Vec2::new(icon_x, icon_y));
                                graphics.append(&scene, Some(transform));
                            }
                        }
                        nptk_services::icon::CachedIcon::Path(_) => {
                            let icon_color = theme
                                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                                .unwrap_or(Color::from_rgb8(150, 150, 150));
                            
                            let fallback_color = if entry.file_type == FileType::Directory {
                                icon_color.with_alpha(0.6)
                            } else {
                                icon_color.with_alpha(0.4)
                            };
                            
                            graphics.fill(
                                Fill::NonZero,
                                Affine::IDENTITY,
                                &Brush::Solid(fallback_color),
                                None,
                                &icon_rect.to_path(0.1),
                            );
                        }
                    }
                } else {
                    let icon_color = theme
                        .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                        .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                        .unwrap_or(Color::from_rgb8(150, 150, 150));
                    
                    let fallback_color = if entry.file_type == FileType::Directory {
                        icon_color.with_alpha(0.6)
                    } else {
                        icon_color.with_alpha(0.4)
                    };
                    
                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(fallback_color),
                        None,
                        &icon_rect.to_path(0.1),
                    );
                }
            }
            
            // Draw text
            let text_color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                .unwrap_or(Color::BLACK);
                
            let transform = Affine::translate((
                row_rect.x0 + 35.0,
                row_rect.y0 + 5.0,
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

impl Widget for FileListContent {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "FileListContent")
    }
    
    fn layout_style(&self) -> StyleNode {
        let view_mode = *self.view_mode.get();
        let count = self.entries.get().len();
        
        let height = if view_mode == FileListViewMode::Icon {
            let icon_size = *self.icon_size.get();
            let (columns, _, cell_height) = self.calculate_icon_view_layout(1000.0, icon_size); // Use large width for calculation
            let rows = (count as f32 / columns as f32).ceil();
            (rows * cell_height + self.icon_view_padding * 2.0).max(100.0)
        } else {
            (count as f32 * self.item_height).max(100.0)
        };
        
        StyleNode {
            style: LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::length(height)),
                ..Default::default()
            },
            children: vec![],
        }
    }
    
    fn update(&mut self, layout: &LayoutNode, context: AppContext, info: &mut AppInfo) -> Update {
        let mut update = Update::empty();
        
        // Poll thumbnail events
        if let Ok(mut rx) = self.thumbnail_event_rx.try_lock() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ThumbnailEvent::ThumbnailReady { entry_path, .. } => {
                        // Thumbnail is ready, invalidate cache and trigger redraw
                        log::debug!("Thumbnail ready for {:?}", entry_path);
                        let mut pending = self.pending_thumbnails.lock().unwrap();
                        pending.remove(&entry_path);
                        update.insert(Update::DRAW);
                    }
                    ThumbnailEvent::ThumbnailFailed { entry_path, error, .. } => {
                        log::warn!("Thumbnail generation failed for {:?}: {}", entry_path, error);
                        let mut pending = self.pending_thumbnails.lock().unwrap();
                        pending.remove(&entry_path);
                    }
                }
            }
        }
        
        if let Some(cursor) = info.cursor_pos {
            let local_y = cursor.y as f32 - layout.layout.location.y;
            let local_x = cursor.x as f32 - layout.layout.location.x;
            
            // Check bounds
            if local_x >= 0.0 && local_x < layout.layout.size.width &&
               local_y >= 0.0 && local_y < layout.layout.size.height 
            {
                let entries = self.entries.get();
                let view_mode = *self.view_mode.get();
                
                // Calculate index based on view mode
                let index = if view_mode == FileListViewMode::Icon {
                    // For icon view, calculate grid position
                    let icon_size = *self.icon_size.get();
                    let (columns, cell_width, cell_height) = self.calculate_icon_view_layout(
                        layout.layout.size.width,
                        icon_size,
                    );
                    let col = (local_x / cell_width).floor() as usize;
                    let row = (local_y / cell_height).floor() as usize;
                    let idx = row * columns + col;
                    if idx < entries.len() { Some(idx) } else { None }
                } else {
                    // For list view, use row-based calculation
                    let idx = (local_y / self.item_height) as usize;
                    if idx < entries.len() { Some(idx) } else { None }
                };
                
                if let Some(index) = index {
                    let entry = &entries[index];
                    let ctrl_pressed = info.modifiers.control_key();
                    let shift_pressed = info.modifiers.shift_key();
                    
                    for (_, btn, el) in &info.buttons {
                        if *btn == MouseButton::Left && *el == ElementState::Pressed {
                            let mut selected = self.selected_paths.get().clone();
                            let is_currently_selected = selected.contains(&entry.path);
                            
                            if shift_pressed {
                                // Shift+Click: Select range from anchor to clicked index
                                let anchor = self.anchor_index.unwrap_or(0);
                                let start = anchor.min(index);
                                let end = anchor.max(index);
                                let range_paths: Vec<PathBuf> = entries[start..=end]
                                    .iter()
                                    .map(|e| e.path.clone())
                                    .collect();
                                
                                // Merge with existing selection if Ctrl is also pressed
                                if ctrl_pressed {
                                    let mut selected_set: HashSet<PathBuf> = selected.iter().cloned().collect();
                                    for path in range_paths {
                                        selected_set.insert(path);
                                    }
                                    selected = selected_set.into_iter().collect();
                                } else {
                                    selected = range_paths;
                                }
                            } else if ctrl_pressed {
                                // Ctrl+Click: Toggle selection
                                if is_currently_selected {
                                    selected.retain(|p| p != &entry.path);
                                } else {
                                    selected.push(entry.path.clone());
                                }
                                self.anchor_index = Some(index);
                            } else {
                                // Single Click: Clear and select only this item
                                selected = vec![entry.path.clone()];
                                self.anchor_index = Some(index);
                            }
                            
                            self.selected_paths.set(selected);
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
                                            self.selected_paths.set(Vec::new());
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
        let view_mode = *self.view_mode.get();
        
        if view_mode == FileListViewMode::Icon {
            self.render_icon_view(graphics, theme, layout, info);
        } else {
            self.render_list_view(graphics, theme, layout, info            );
        }
    }
}

impl FileListContent {
    fn render_icon_view(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
    ) {
        let entries = self.entries.get();
        let selected_paths = self.selected_paths.get();
        let selected_set: HashSet<&PathBuf> = selected_paths.iter().collect();
        let entry_count = entries.len();
        
        // Draw background
        let bg_rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );
        
        let bg_color = theme
            .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackground)
            .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackground))
            .unwrap_or_else(|| theme.window_background());
        
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
        
        let icon_size = *self.icon_size.get();
        let (columns, cell_width, cell_height) = self.calculate_icon_view_layout(
            layout.layout.size.width,
            icon_size,
        );
        
        for (i, entry) in entries.iter().enumerate() {
            let (x, y) = self.get_icon_position(i, columns, cell_width, cell_height);
            let cell_rect = Rect::new(
                layout.layout.location.x as f64 + x as f64,
                layout.layout.location.y as f64 + y as f64,
                layout.layout.location.x as f64 + x as f64 + cell_width as f64,
                layout.layout.location.y as f64 + y as f64 + cell_height as f64,
            );
            
            // Check for hover state
            let is_hovered = if let Some(cursor) = info.cursor_pos {
                let cursor_x = cursor.x as f64;
                let cursor_y = cursor.y as f64;
                cursor_x >= cell_rect.x0 && cursor_x < cell_rect.x1 &&
                cursor_y >= cell_rect.y0 && cursor_y < cell_rect.y1
            } else {
                false
            };
            
            // Draw hover background (if not selected)
            if is_hovered && !selected_set.contains(&entry.path) {
                let hover_color = theme
                    .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorMenuHovered)
                    .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorMenuHovered))
                    .unwrap_or_else(|| Color::from_rgb8(240, 240, 240));
                
                let hover_rect = RoundedRect::new(
                    cell_rect.x0,
                    cell_rect.y0,
                    cell_rect.x1,
                    cell_rect.y1,
                    RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
                );
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(hover_color.with_alpha(0.5)),
                    None,
                    &hover_rect.to_path(0.1),
                );
            }
            
            // Draw selection background
            if selected_set.contains(&entry.path) {
                let color = theme
                    .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                    .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackgroundSelected))
                    .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));
                
                let selection_rect = RoundedRect::new(
                    cell_rect.x0,
                    cell_rect.y0,
                    cell_rect.x1,
                    cell_rect.y1,
                    RoundedRectRadii::new(4.0, 4.0, 4.0, 4.0),
                );
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(0.3)),
                    None,
                    &selection_rect.to_path(0.1),
                );
            }
            
            // Calculate icon position (centered in cell)
            let icon_x = cell_rect.x0 + (cell_width as f64 - icon_size as f64) / 2.0;
            let icon_y = cell_rect.y0 + self.icon_view_spacing as f64;
            let icon_rect = Rect::new(
                icon_x,
                icon_y,
                icon_x + icon_size as f64,
                icon_y + icon_size as f64,
            );
            
            // Try to get thumbnail first, fall back to icon
            let mut use_thumbnail = false;
            if let Some(thumbnail_path) = self.thumbnail_provider.get_thumbnail(entry, icon_size) {
                if let Ok(Some(cached_thumb)) = self.thumbnail_cache.load_or_get(&thumbnail_path, icon_size) {
                    use nptk_core::vg::peniko::{Blob, ImageBrush, ImageData, ImageFormat, ImageAlphaType};
                    let image_data = ImageData {
                        data: Blob::from(cached_thumb.data.as_ref().clone()),
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width: cached_thumb.width,
                        height: cached_thumb.height,
                    };
                    let image_brush = ImageBrush::new(image_data);
                    let scale_x = icon_size as f64 / (cached_thumb.width as f64);
                    let scale_y = icon_size as f64 / (cached_thumb.height as f64);
                    let scale = scale_x.min(scale_y);
                    let transform = Affine::scale_non_uniform(scale, scale)
                        .then_translate(Vec2::new(icon_x, icon_y));
                    if let Some(scene) = graphics.as_scene_mut() {
                        scene.draw_image(&image_brush, transform);
                    }
                    use_thumbnail = true;
                }
            }
            
            // If no thumbnail, use icon
            if !use_thumbnail {
                // Request thumbnail generation if supported
                if self.thumbnail_provider.is_supported(entry) {
                    let mut pending = self.pending_thumbnails.lock().unwrap();
                    if !pending.contains(&entry.path) {
                        if let Ok(()) = self.thumbnail_provider.request_thumbnail(entry, icon_size) {
                            pending.insert(entry.path.clone());
                        }
                    }
                }
                
                // Get icon for this entry
                let cache_key = (entry.path.clone(), icon_size);
                let cached_icon = {
                    let mut cache = self.icon_cache.lock().unwrap();
                    if let Some(icon) = cache.get(&cache_key) {
                        icon.clone()
                    } else {
                        let icon = self.icon_registry.get_file_icon(entry, icon_size);
                        cache.insert(cache_key.clone(), icon.clone());
                        icon
                    }
                };
                
                if let Some(icon) = cached_icon {
                    match icon {
                        nptk_services::icon::CachedIcon::Image { data, width, height } => {
                            use nptk_core::vg::peniko::{Blob, ImageBrush, ImageData, ImageFormat, ImageAlphaType};
                            let image_data = ImageData {
                                data: Blob::from(data.as_ref().clone()),
                                format: ImageFormat::Rgba8,
                                alpha_type: ImageAlphaType::Alpha,
                                width,
                                height,
                            };
                            let image_brush = ImageBrush::new(image_data);
                            let scale_x = icon_size as f64 / (width as f64);
                            let scale_y = icon_size as f64 / (height as f64);
                            let scale = scale_x.min(scale_y);
                            let transform = Affine::scale_non_uniform(scale, scale)
                                .then_translate(Vec2::new(icon_x, icon_y));
                            if let Some(scene) = graphics.as_scene_mut() {
                                scene.draw_image(&image_brush, transform);
                            }
                        }
                        nptk_services::icon::CachedIcon::Svg(svg_source) => {
                            use vello_svg::usvg::{Tree, Options, ShapeRendering, TextRendering, ImageRendering};
                            if let Ok(tree) = Tree::from_str(
                                svg_source.as_str(),
                                &Options {
                                    shape_rendering: ShapeRendering::GeometricPrecision,
                                    text_rendering: TextRendering::OptimizeLegibility,
                                    image_rendering: ImageRendering::OptimizeSpeed,
                                    ..Default::default()
                                },
                            ) {
                                let scene = vello_svg::render_tree(&tree);
                                let svg_size = tree.size();
                                let scale_x = icon_size as f64 / svg_size.width() as f64;
                                let scale_y = icon_size as f64 / svg_size.height() as f64;
                                let scale = scale_x.min(scale_y);
                                let transform = Affine::scale_non_uniform(scale, scale)
                                    .then_translate(Vec2::new(icon_x, icon_y));
                                graphics.append(&scene, Some(transform));
                            }
                        }
                        nptk_services::icon::CachedIcon::Path(_) => {
                            let icon_color = theme
                                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                                .unwrap_or(Color::from_rgb8(150, 150, 150));
                            
                            let fallback_color = if entry.file_type == FileType::Directory {
                                icon_color.with_alpha(0.6)
                            } else {
                                icon_color.with_alpha(0.4)
                            };
                            
                            graphics.fill(
                                Fill::NonZero,
                                Affine::IDENTITY,
                                &Brush::Solid(fallback_color),
                                None,
                                &icon_rect.to_path(0.1),
                            );
                        }
                    }
                } else {
                    // Fallback to colored rectangle
                    let icon_color = theme
                        .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                        .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                        .unwrap_or(Color::from_rgb8(150, 150, 150));
                    
                    let fallback_color = if entry.file_type == FileType::Directory {
                        icon_color.with_alpha(0.6)
                    } else {
                        icon_color.with_alpha(0.4)
                    };
                    
                    graphics.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(fallback_color),
                        None,
                        &icon_rect.to_path(0.1),
                    );
                }
            }
            
            // Draw filename below icon (centered)
            let text_color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                .unwrap_or(Color::BLACK);
            
            let text_y = icon_rect.y1 + 4.0;
            let text_width = cell_width as f32 - self.icon_view_spacing * 2.0;
            let text_x = cell_rect.x0 + (cell_width as f64 - text_width as f64) / 2.0;
            
            let transform = Affine::translate((text_x, text_y));
            
            self.text_render_context.render_text(
                &mut info.font_context,
                graphics,
                &entry.name,
                None,
                12.0, // Smaller font for icon view
                Brush::Solid(text_color),
                transform,
                true,
                Some(text_width),
            );
        }
    }
}
