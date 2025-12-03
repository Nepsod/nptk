use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::font_ctx::FontContext;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{Affine, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke, Vec2};
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
    /// Compact view (Tiles view: Icon left, Text right, grid layout)
    Compact,
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
    
    // Drag selection state
    drag_start: Option<Point>,
    current_drag_pos: Option<Point>,
    is_dragging: bool,
    
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
            
            drag_start: None,
            current_drag_pos: None,
            is_dragging: false,
            
            icon_view_padding: 2.0, // padding around the icons
            icon_view_spacing: 22.0, // spacing between icons
            icon_view_text_height: 50.0, // Increased to accommodate 2-3 lines of wrapped text
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
        // Calculate cell height: icon + spacing + text area (2-3 lines)
        // Text area: ~37.5px for 2-3 lines, plus spacing between icon and text
        let text_area_height = 12.0 * 1.25 * 2.5; // ~37.5px for 2-3 lines
        let cell_height = icon_size_f + 4.0 + text_area_height + self.icon_view_spacing; // icon + gap + text + bottom spacing
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

    fn update_drag_selection(&mut self, selection_rect: Rect, toggle: bool, layout_width: f32) {
        let entries = self.entries.get();
        let view_mode = *self.view_mode.get();
        let icon_size = *self.icon_size.get();
        
        let mut new_selection = if toggle {
            self.selected_paths.get().clone()
        } else {
            Vec::new()
        };
        
        // Helper to check intersection
        let check_intersection = |item_rect: Rect| -> bool {
            let intersection = selection_rect.intersect(item_rect);
            intersection.width() > 0.0 && intersection.height() > 0.0
        };

        if view_mode == FileListViewMode::Icon {
            let (columns, cell_width, cell_height) = self.calculate_icon_view_layout(layout_width, icon_size);
            
            for (i, entry) in entries.iter().enumerate() {
                let (x, y) = self.get_icon_position(i, columns, cell_width, cell_height);
                // We use the full cell rect for intersection to make it easier to select
                let cell_rect = Rect::new(
                    x as f64, 
                    y as f64, 
                    (x + cell_width) as f64, 
                    (y + cell_height) as f64
                );
                
                if check_intersection(cell_rect) {
                    if !new_selection.contains(&entry.path) {
                        new_selection.push(entry.path.clone());
                    }
                } else if !toggle {
                     // If not toggling, we strictly set selection to what's in the rect
                     // So if it was selected but not in rect, it's removed (already handled by init empty Vec)
                }
            }
        } else if view_mode == FileListViewMode::Compact {
            let (columns, cell_width, cell_height, spacing) = self.calculate_compact_view_layout(layout_width);
            
            for (i, entry) in entries.iter().enumerate() {
                let col = i % columns;
                let row = i / columns;
                let x = self.icon_view_padding + col as f32 * (cell_width + spacing);
                let y = self.icon_view_padding + row as f32 * (cell_height + spacing);
                
                let cell_rect = Rect::new(
                    x as f64,
                    y as f64,
                    (x + cell_width) as f64,
                    (y + cell_height) as f64
                );
                
                if check_intersection(cell_rect) {
                    if !new_selection.contains(&entry.path) {
                        new_selection.push(entry.path.clone());
                    }
                }
            }
        } else {
             // List view
             for (i, entry) in entries.iter().enumerate() {
                 let y = i as f32 * self.item_height;
                 let row_rect = Rect::new(
                     0.0,
                     y as f64,
                     layout_width as f64,
                     (y + self.item_height) as f64
                 );
                 
                 if check_intersection(row_rect) {
                     if !new_selection.contains(&entry.path) {
                         new_selection.push(entry.path.clone());
                     }
                 }
             }
        }
        
        self.selected_paths.set(new_selection);
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
        } else if view_mode == FileListViewMode::Compact {
            let (columns, _, cell_height, spacing) = self.calculate_compact_view_layout(1000.0);
            let rows = (count as f32 / columns as f32).ceil();
            // Height = rows * cell + (rows - 1) * spacing + padding
            // Approx: rows * (cell + spacing) - spacing + padding
            (rows * (cell_height + spacing) - spacing + self.icon_view_padding * 2.0).max(100.0)
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
                // let entries = self.entries.get(); // Removed to avoid holding borrow
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
                    
                    // Scope entries borrow
                    let entry_opt = {
                        let entries = self.entries.get();
                        if idx < entries.len() {
                            Some(entries[idx].clone())
                        } else {
                            None
                        }
                    };

                    if let Some(entry) = entry_opt {
                        // Tight hit testing for Icon view
                        let cell_x = col as f32 * cell_width;
                        let cell_y = row as f32 * cell_height;
                        let cell_rect = Rect::new(
                            cell_x as f64,
                            cell_y as f64,
                            (cell_x + cell_width) as f64,
                            (cell_y + cell_height) as f64,
                        );
                        
                        // Check if selected (needed for label height calculation)
                        let is_selected = {
                            let selected = self.selected_paths.get();
                            selected.contains(&entry.path)
                        };
                        
                        let (icon_rect, label_rect, _, _) = self.get_icon_item_layout(
                            &mut info.font_context,
                            &entry,
                            cell_rect,
                            cell_width,
                            icon_size as f32,
                            is_selected,
                        );
                        
                        let cursor_x = local_x as f64;
                        let cursor_y = local_y as f64;
                        
                        if (cursor_x >= icon_rect.x0 && cursor_x < icon_rect.x1 &&
                            cursor_y >= icon_rect.y0 && cursor_y < icon_rect.y1) ||
                           (cursor_x >= label_rect.x0 && cursor_x < label_rect.x1 &&
                            cursor_y >= label_rect.y0 && cursor_y < label_rect.y1)
                        {
                            Some(idx)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else if view_mode == FileListViewMode::Compact {
                    // For compact view
                    let (columns, cell_width, cell_height, spacing) = self.calculate_compact_view_layout(
                        layout.layout.size.width,
                    );
                    // Account for spacing in hit testing
                    // x = padding + col * (width + spacing)
                    // col = (x - padding) / (width + spacing)
                    let col = ((local_x - self.icon_view_padding) / (cell_width + spacing)).floor() as usize;
                    let row = ((local_y - self.icon_view_padding) / (cell_height + spacing)).floor() as usize;
                    
                    // Check if within cell bounds (exclude spacing gap)
                    let cell_x = self.icon_view_padding + col as f32 * (cell_width + spacing);
                    let cell_y = self.icon_view_padding + row as f32 * (cell_height + spacing);
                    
                    if local_x >= cell_x && local_x < cell_x + cell_width &&
                       local_y >= cell_y && local_y < cell_y + cell_height {
                        let idx = row * columns + col;
                        
                        // Scope the borrow of entries to clone the needed entry
                        let entry_opt = {
                            let entries = self.entries.get();
                            if idx < entries.len() {
                                Some(entries[idx].clone())
                            } else {
                                None
                            }
                        };

                        if let Some(entry) = entry_opt { 
                            // Tight hit testing
                            let cell_rect = Rect::new(
                                cell_x as f64,
                                cell_y as f64,
                                (cell_x + cell_width) as f64,
                                (cell_y + cell_height) as f64,
                            );
                            
                            let (icon_rect, label_rect) = self.get_compact_item_layout(
                                &mut info.font_context,
                                &entry,
                                cell_rect,
                                cell_height,
                                cell_width,
                            );
                            
                            let cursor_x = local_x as f64;
                            let cursor_y = local_y as f64;
                            
                            if (cursor_x >= icon_rect.x0 && cursor_x < icon_rect.x1 &&
                                cursor_y >= icon_rect.y0 && cursor_y < icon_rect.y1) ||
                               (cursor_x >= label_rect.x0 && cursor_x < label_rect.x1 &&
                                cursor_y >= label_rect.y0 && cursor_y < label_rect.y1)
                            {
                                Some(idx)
                            } else {
                                None
                            }
                        } else { 
                            None 
                        }
                    } else {
                        None
                    }
                } else {
                    // For list view, use row-based calculation
                    let idx = (local_y / self.item_height) as usize;
                    let entries = self.entries.get();
                    if idx < entries.len() { Some(idx) } else { None }
                };
                
                if let Some(index) = index {
                    // Extract data needed for selection logic to avoid holding entries borrow
                    let (target_path, range_paths, file_type) = {
                        let entries = self.entries.get();
                        if index < entries.len() {
                            let entry = &entries[index];
                            let target = entry.path.clone();
                            let ftype = entry.file_type;
                            
                            let range = if info.modifiers.shift_key() {
                                let anchor = self.anchor_index.unwrap_or(0);
                                let start = anchor.min(index);
                                let end = anchor.max(index);
                                Some(entries[start..=end].iter().map(|e| e.path.clone()).collect::<Vec<_>>())
                            } else {
                                None
                            };
                            (Some(target), range, Some(ftype))
                        } else {
                            (None, None, None)
                        }
                    };

                    if let Some(target_path) = target_path {
                        // Debug: Log key events
        for (_, key_event) in &info.keys {
            if key_event.state == ElementState::Pressed {
                println!("Key Pressed: {:?} (Logical: {:?})", key_event.physical_key, key_event.logical_key);
            }
        }

        let ctrl_pressed = info.modifiers.control_key();
        let shift_pressed = info.modifiers.shift_key();
                        
                        for (_, btn, el) in &info.buttons {
                            if *btn == MouseButton::Left && *el == ElementState::Pressed {
                                println!("Item Click: index={:?}, Ctrl={}, Shift={}", index, info.modifiers.control_key(), info.modifiers.shift_key());
                                let mut selected = self.selected_paths.get().clone();
                                let is_currently_selected = selected.contains(&target_path);
                                
                                if let Some(range_paths) = &range_paths {
                                    // Shift+Click: Select range
                                    // Merge with existing selection if Ctrl is also pressed
                                    if ctrl_pressed {
                                        let mut selected_set: HashSet<PathBuf> = selected.iter().cloned().collect();
                                        for path in range_paths {
                                            selected_set.insert(path.clone());
                                        }
                                        selected = selected_set.into_iter().collect();
                                    } else {
                                        selected = range_paths.clone();
                                    }
                                } else if ctrl_pressed {
                                    // Ctrl+Click: Toggle selection
                                    if is_currently_selected {
                                        selected.retain(|p| p != &target_path);
                                    } else {
                                        selected.push(target_path.clone());
                                    }
                                    self.anchor_index = Some(index);
                                } else {
                                    // Single Click: Clear and select only this item
                                    selected = vec![target_path.clone()];
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
                                            if let Some(ftype) = file_type {
                                                if ftype == FileType::Directory {
                                                    // Navigate
                                                    self.current_path.set(target_path.clone());
                                                    let _ = self.fs_model.refresh(&target_path);
                                                    self.selected_paths.set(Vec::new());
                                                    update.insert(Update::LAYOUT);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                self.last_click_time = Some(now);
                                self.last_click_index = Some(index);
                            }
                        }
                    } else {
                        // Clicked on empty space
                        for (_, btn, el) in &info.buttons {
                            if *btn == MouseButton::Left && *el == ElementState::Pressed {
                                println!("Empty Space Click (Index Valid but Target None): Ctrl={}", info.modifiers.control_key());
                                // Start dragging
                                self.drag_start = Some(Point::new(local_x as f64, local_y as f64));
                                self.current_drag_pos = Some(Point::new(local_x as f64, local_y as f64));
                                self.is_dragging = false; // Will become true on move
                                
                                // Clear selection if Ctrl is not pressed
                                if !info.modifiers.control_key() {
                                    self.selected_paths.set(Vec::new());
                                    update.insert(Update::DRAW);
                                }
                            }
                        }
                    }
                } else {
                    // Index is None (empty space)
                    for (_, btn, el) in &info.buttons {
                        if *btn == MouseButton::Left && *el == ElementState::Pressed {
                            println!("Empty Space Click (Index None): Ctrl={}", info.modifiers.control_key());
                            // Start dragging
                            self.drag_start = Some(Point::new(local_x as f64, local_y as f64));
                            self.current_drag_pos = Some(Point::new(local_x as f64, local_y as f64));
                            self.is_dragging = false;
                            
                            if !info.modifiers.control_key() {
                                self.selected_paths.set(Vec::new());
                                update.insert(Update::DRAW);
                            }
                        }
                    }
                }
                
                // Handle Dragging
                if let Some(start_pos) = self.drag_start {
                    // Check if mouse released
                    let mut released = false;
                    for (_, btn, el) in &info.buttons {
                        if *btn == MouseButton::Left && *el == ElementState::Released {
                            released = true;
                            break;
                        }
                    }
                    
                    if released {
                        self.drag_start = None;
                        self.current_drag_pos = None;
                        self.is_dragging = false;
                        update.insert(Update::DRAW);
                    } else {
                        // Update drag position
                        let current_pos = Point::new(local_x as f64, local_y as f64);
                        self.current_drag_pos = Some(current_pos);
                        
                        // Check if we moved enough to consider it a drag
                        if !self.is_dragging {
                            let dx = current_pos.x - start_pos.x;
                            let dy = current_pos.y - start_pos.y;
                            if dx.abs() > 5.0 || dy.abs() > 5.0 {
                                self.is_dragging = true;
                            }
                        }
                        
                        if self.is_dragging {
                            // Calculate selection rect
                            let min_x = start_pos.x.min(current_pos.x);
                            let min_y = start_pos.y.min(current_pos.y);
                            let max_x = start_pos.x.max(current_pos.x);
                            let max_y = start_pos.y.max(current_pos.y);
                            
                            let selection_rect = Rect::new(min_x, min_y, max_x, max_y);
                            
                            // Update selection
                            self.update_drag_selection(
                                selection_rect, 
                                info.modifiers.control_key(),
                                layout.layout.size.width
                            );
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
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        let view_mode = *self.view_mode.get();
        
        if view_mode == FileListViewMode::Icon {
            self.render_icon_view(graphics, theme, layout, info);
        } else if view_mode == FileListViewMode::Compact {
            self.render_compact_view(graphics, theme, layout, info);
        } else {
            self.render_list_view(graphics, theme, layout, info);
        }
        
        // Draw drag selection rectangle
        if self.is_dragging {
            if let (Some(start), Some(current)) = (self.drag_start, self.current_drag_pos) {
                let min_x = start.x.min(current.x);
                let min_y = start.y.min(current.y);
                let max_x = start.x.max(current.x);
                let max_y = start.y.max(current.y);
                
                let rect = Rect::new(min_x, min_y, max_x, max_y);
                
                let selection_color = theme
                    .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                    .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackgroundSelected))
                    .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));
                    
                // Draw selection fill
                graphics.fill(
                    Fill::NonZero,
                    Affine::translate((layout.layout.location.x as f64, layout.layout.location.y as f64)),
                    &Brush::Solid(selection_color.with_alpha(0.2)),
                    None,
                    &rect.to_path(0.1),
                );
                
                // Draw selection border
                graphics.stroke(
                    &Stroke::new(1.0),
                    Affine::translate((layout.layout.location.x as f64, layout.layout.location.y as f64)),
                    &Brush::Solid(selection_color.with_alpha(0.8)),
                    None,
                    &rect.to_path(0.1),
                );
            }
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
        let entries = self.entries.get().clone();
        let selected_paths = self.selected_paths.get().clone();
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
        
        // Pass 1: Render unselected items
        for (i, entry) in entries.iter().enumerate() {
            if !selected_set.contains(&entry.path) {
                self.render_icon_item(
                    graphics, theme, layout, info, 
                    i, entry, columns, cell_width, cell_height, icon_size, 
                    false
                );
            }
        }

        // Pass 2: Render selected items (to draw on top)
        for (i, entry) in entries.iter().enumerate() {
            if selected_set.contains(&entry.path) {
                self.render_icon_item(
                    graphics, theme, layout, info, 
                    i, entry, columns, cell_width, cell_height, icon_size, 
                    true
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_icon_item(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        i: usize,
        entry: &FileEntry,
        columns: usize,
        cell_width: f32,
        cell_height: f32,
        icon_size: u32,
        is_selected: bool,
    ) {
        let (x, y) = self.get_icon_position(i, columns, cell_width, cell_height);
        let cell_rect = Rect::new(
            layout.layout.location.x as f64 + x as f64,
            layout.layout.location.y as f64 + y as f64,
            layout.layout.location.x as f64 + x as f64 + cell_width as f64,
            layout.layout.location.y as f64 + y as f64 + cell_height as f64,
        );
        
        // Calculate icon position (centered in cell)
        let icon_x = cell_rect.x0 + (cell_width as f64 - icon_size as f64) / 2.0;
        let icon_y = cell_rect.y0 + self.icon_view_spacing as f64;
        let icon_rect = Rect::new(
            icon_x,
            icon_y,
            icon_x + icon_size as f64,
            icon_y + icon_size as f64,
        );
        // https://learn.microsoft.com/en-us/windows/win32/controls/lvm-getitemrect
        // Classic Windows approach: Calculate icon and label rectangles separately, then union them
        // Step 1: Measure text layout to get actual line count and width
        let font_size = 12.0;
        // Calculate layout using helper
        let (icon_rect, label_rect, display_text, max_text_width) = self.get_icon_item_layout(
            &mut info.font_context,
            entry,
            cell_rect,
            cell_width,
            icon_size as f32,
            is_selected,
        );
        
        // Step 3: Drawing
        
        // 1. Draw Label Backgrounds (Hover/Selection) - behind text
        // Check for hover state (check if cursor is in icon OR label area)
        let is_hovered = if let Some(cursor) = info.cursor_pos {
            let cursor_x = cursor.x as f64;
            let cursor_y = cursor.y as f64;
            // Check if cursor is in icon rectangle
            let in_icon = cursor_x >= icon_rect.x0 && cursor_x < icon_rect.x1 &&
                            cursor_y >= icon_rect.y0 && cursor_y < icon_rect.y1;
            // Check if cursor is in label rectangle
            let in_label = cursor_x >= label_rect.x0 && cursor_x < label_rect.x1 &&
                            cursor_y >= label_rect.y0 && cursor_y < label_rect.y1;
            in_icon || in_label
        } else {
            false
        };
        
        if is_hovered && !is_selected {
            let hover_color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorMenuHovered)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorMenuHovered))
                .unwrap_or_else(|| Color::from_rgb8(240, 240, 240));
            
            // Draw label hover rectangle
            let label_hover_rect = RoundedRect::new(
                label_rect.x0,
                label_rect.y0,
                label_rect.x1,
                label_rect.y1,
                RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0),
            );
            
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(hover_color),
                None,
                &label_hover_rect.to_path(0.1),
            );
        }
        
        if is_selected {
            let color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackgroundSelected))
                .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));
            
            // Draw label selection rectangle
            let label_selection_rect = RoundedRect::new(
                label_rect.x0,
                label_rect.y0,
                label_rect.x1,
                label_rect.y1,
                RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0),
            );
            
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(color.with_alpha(0.9)),
                None,
                &label_selection_rect.to_path(0.1),
            );
        }
        
        // 2. Draw Icon
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
                            .then_translate(Vec2::new(icon_rect.x0, icon_rect.y0));
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
                                .then_translate(Vec2::new(icon_rect.x0, icon_rect.y0));
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

        // 3. Draw Icon Overlays (Hover/Selection) - on top of icon (tint)
        if is_hovered && !is_selected {
            let hover_color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorMenuHovered)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorMenuHovered))
                .unwrap_or_else(|| Color::from_rgb8(240, 240, 240));
            
            // Draw icon hover rectangle (overlay)
            let icon_hover_rect = RoundedRect::new(
                icon_rect.x0,
                icon_rect.y0,
                icon_rect.x1,
                icon_rect.y1,
                RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0),
            );
            
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(hover_color.with_alpha(0.5)), // Tint alpha
                None,
                &icon_hover_rect.to_path(0.1),
            );
        }

        if is_selected {
            let color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorBackgroundSelected))
                .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));
            
            // Draw icon selection rectangle (overlay)
            let icon_selection_rect = RoundedRect::new(
                icon_rect.x0,
                icon_rect.y0,
                icon_rect.x1,
                icon_rect.y1,
                RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0),
            );
            
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(color.with_alpha(0.5)), // Tint alpha
                None,
                &icon_selection_rect.to_path(0.1),
            );
        }
        
        // Draw filename in label rectangle
        let text_color = theme
            .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
            .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
            .unwrap_or(Color::BLACK);
        
        // Text position: Start at the left edge of the max_text_width area.
        // We use max_text_width as the wrap width, and ask Parley to center align.
        // So we must position the "box" we are drawing into at the center of the cell.
        let text_x = cell_rect.x0 + (cell_width as f64 - max_text_width as f64) / 2.0;
        let text_y = label_rect.y0 + 2.0; // label_padding
        
        // Clipping:
        // - Unselected: Clip to cell bounds (minus padding) to prevent overlap.
        // - Selected: Clip horizontally to cell, but extend bottom to allow full text visibility.
        let text_clip_rect = if is_selected {
            Rect::new(
                cell_rect.x0 + self.icon_view_padding as f64,
                cell_rect.y0 + self.icon_view_padding as f64,
                cell_rect.x1 - self.icon_view_padding as f64,
                label_rect.y1 + self.icon_view_padding as f64, // Extend to full label height
            )
        } else {
            Rect::new(
                cell_rect.x0 + self.icon_view_padding as f64,
                cell_rect.y0 + self.icon_view_padding as f64,
                cell_rect.x1 - self.icon_view_padding as f64,
                cell_rect.y1 - self.icon_view_padding as f64,
            )
        };
        
        // Apply clipping for text rendering to prevent overflow
        use nptk_core::vg::peniko::Mix;
        #[allow(deprecated)]
        graphics.push_layer(Mix::Clip, 1.0, Affine::IDENTITY, &text_clip_rect.to_path(0.1));
        
        let transform = Affine::translate((text_x, text_y));
        
        // Render text with wrapping enabled.
        // We use max_text_width as the wrap_width. This ensures long filenames wrap at the cell boundary.
        let wrap_width = Some(max_text_width);
        
        // Render text with optional line limit.
        // Dolphin-like behavior:
        // - Not Selected: Limit to 2 lines.
        // - Selected: Show all lines (unlimited).
        let max_lines = if !is_selected {
            Some(2) 
        } else {
            None 
        };
        
        // Always center align.
        // Parley will center the text within wrap_width (which is max_text_width).
        // Since we positioned text_x at the start of max_text_width area, the text will be visually centered in the cell.
        let center_align = true;

        self.text_render_context.render_text_with_max_lines(
            &mut info.font_context,
            graphics,
            &display_text,
            None,
            font_size,
            Brush::Solid(text_color),
            transform,
            true,
            wrap_width,
            max_lines,
            center_align, 
        );
        
        // If not selected and text was truncated (more than 2 lines), draw "..." indicator.
        // This applies to all names (with or without special characters).
        // Manual ellipsis drawing removed as variables are no longer available.
        // Truncation is handled by render_text_with_max_lines and clipping.
        
        // Pop clipping layer
        graphics.pop_layer();
    }

    fn calculate_compact_view_layout(&self, width: f32) -> (usize, f32, f32, f32) {
        let cell_width = 250.0; // Fixed width for compact tiles
        let cell_height = 60.0; // Fixed height for compact tiles
        let spacing = 10.0;     // Spacing between tiles
        
        let available_width = width - self.icon_view_padding * 2.0;
        let columns = ((available_width + spacing) / (cell_width + spacing)).floor() as usize;
        let columns = columns.max(1);
        
        (columns, cell_width, cell_height, spacing)
    }

    fn get_compact_item_layout(
        &mut self,
        font_cx: &mut FontContext,
        entry: &FileEntry,
        cell_rect: Rect,
        cell_height: f32,
        cell_width: f32,
    ) -> (Rect, Rect) {
        // Define Icon area
        let icon_size = 32.0f32;
        let icon_padding = 8.0f32;
        let icon_x = cell_rect.x0 + icon_padding as f64;
        let icon_y = cell_rect.y0 + (cell_height as f64 - icon_size as f64) / 2.0;
        let icon_rect = Rect::new(icon_x, icon_y, icon_x + icon_size as f64, icon_y + icon_size as f64);
        
        let text_x = icon_x + icon_size as f64 + 10.0;
        let text_y = cell_rect.y0 + 12.0;
        let max_text_width = cell_width - (icon_size + icon_padding * 2.0 + 10.0);
        
        // Measure text to determine label width
        let font_size = 14.0;
        let (text_width, line_count) = self.text_render_context.measure_text_layout(
            font_cx,
            &entry.name,
            font_size,
            Some(max_text_width as f32),
        );
        
        let display_lines = line_count.min(2);
        let text_height = display_lines as f32 * (font_size * 1.2); // Approx height
        
        // Label rect (tight fit around text)
        let label_padding_x = 4.0;
        let label_padding_y = 2.0;
        let label_rect = Rect::new(
            text_x - label_padding_x,
            text_y - label_padding_y,
            text_x + text_width as f64 + label_padding_x,
            text_y + text_height as f64 + label_padding_y
        );
        
        (icon_rect, label_rect)
    }

    fn get_icon_item_layout(
        &mut self,
        font_cx: &mut FontContext,
        entry: &FileEntry,
        cell_rect: Rect,
        cell_width: f32,
        icon_size: f32,
        is_selected: bool,
    ) -> (Rect, Rect, String, f32) {
        // Step 1: Calculate Icon Rectangle
        // Icon is centered horizontally, with padding from top
        let icon_x = cell_rect.x0 + (cell_width as f64 - icon_size as f64) / 2.0;
        let icon_y = cell_rect.y0 + self.icon_view_padding as f64;
        
        let icon_rect = Rect::new(
            icon_x,
            icon_y,
            icon_x + icon_size as f64,
            icon_y + icon_size as f64,
        );
        
        // Step 2: Prepare text for wrapping
        let font_size = 12.0;
        let line_height = font_size * 1.2;
        let max_text_width = (cell_width - self.icon_view_padding * 2.0).max(10.0);
        
        let name = &entry.name;
        let (text_with_breaks, has_natural_breaks) = {
            let is_continuous = name.chars().all(|c| c.is_alphanumeric());
            let mut result = String::with_capacity(name.len() + name.len() / 8);
            let mut segment_len: usize = 0;

            for c in name.chars() {
                result.push(c);
                let is_special = !c.is_alphanumeric() && !c.is_whitespace();
                if is_special {
                    result.push('\u{200B}');
                    segment_len = 0;
                } else if c.is_whitespace() {
                    segment_len = 0;
                } else {
                    segment_len += 1;
                    if is_continuous && segment_len >= 10 {
                        result.push('\u{200B}');
                        segment_len = 0;
                    }
                }
            }
            (result, !is_continuous)
        };
        
        // Measure text layout
        let (measured_width, line_count) = self.text_render_context.measure_text_layout(
            font_cx,
            &text_with_breaks,
            font_size,
            Some(max_text_width),
        );
        
        // Calculate label dimensions
        let label_padding = 2.0;
        let label_spacing = 4.0;
        let label_y_start = icon_rect.y1 + label_spacing;
        
        let displayed_line_count = if is_selected { line_count } else { line_count.min(2) };
        let per_line_height = line_height as f64 + 2.0;
        let label_height = (displayed_line_count as f64 * per_line_height).max(per_line_height);
        
        let max_label_width = (cell_width as f64 - 2.0 * label_padding as f64).max(0.0);
        let base_width = measured_width as f64;
        let label_width = if line_count == 1 && !has_natural_breaks && measured_width > max_text_width {
            (base_width.min(max_text_width as f64 * 1.5)).min(max_label_width)
        } else {
            base_width.min(max_text_width as f64).min(max_label_width)
        };
        
        let label_x = cell_rect.x0 + (cell_width as f64 - label_width) / 2.0;
        let label_y = label_y_start;
        
        let label_rect = Rect::new(
            label_x - label_padding,
            label_y - label_padding,
            label_x + label_width + label_padding,
            label_y + label_height + label_padding,
        );
        
        (icon_rect, label_rect, text_with_breaks, max_text_width)
    }

    fn render_compact_view(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
    ) {
        let entries = self.entries.get().clone();
        let selected_paths = self.selected_paths.get().clone();
        let selected_set: HashSet<&PathBuf> = selected_paths.iter().collect();
        
        let (columns, cell_width, cell_height, spacing) = self.calculate_compact_view_layout(layout.layout.size.width);
        
        for (i, entry) in entries.iter().enumerate() {
            let row = i / columns;
            let col = i % columns;
            
            let x = self.icon_view_padding + col as f32 * (cell_width + spacing);
            let y = self.icon_view_padding + row as f32 * (cell_height + spacing);
            
            let cell_rect = Rect::new(
                x as f64,
                y as f64,
                (x + cell_width) as f64,
                (y + cell_height) as f64,
            );
            
            let is_selected = selected_set.contains(&entry.path);
            
            // Calculate layout
            let (icon_rect, label_rect) = self.get_compact_item_layout(
                &mut info.font_context,
                entry,
                cell_rect,
                cell_height,
                cell_width,
            );
            
            // Check for hover state
            let is_hovered = if let Some(cursor) = info.cursor_pos {
                let cursor_x = cursor.x as f64;
                let cursor_y = cursor.y as f64;
                // Tight hit testing for hover visual
                (cursor_x >= icon_rect.x0 && cursor_x < icon_rect.x1 &&
                 cursor_y >= icon_rect.y0 && cursor_y < icon_rect.y1) ||
                (cursor_x >= label_rect.x0 && cursor_x < label_rect.x1 &&
                 cursor_y >= label_rect.y0 && cursor_y < label_rect.y1)
            } else {
                false
            };
            
            // Extract layout properties for rendering
            let icon_x = icon_rect.x0;
            let icon_y = icon_rect.y0;
            let icon_size = icon_rect.width() as f32;
            let font_size = 14.0;

            // 1. Draw Label Background (Selection/Hover)
            if is_selected || is_hovered {
                let color_prop = if is_selected {
                    nptk_theme::properties::ThemeProperty::ColorBackgroundSelected
                } else {
                    nptk_theme::properties::ThemeProperty::ColorMenuHovered
                };
                
                let color = theme
                    .get_property(self.widget_id(), &color_prop)
                    .or_else(|| theme.get_default_property(&color_prop))
                    .unwrap_or_else(|| if is_selected { Color::from_rgb8(100, 150, 255) } else { Color::from_rgb8(240, 240, 240) });
                
                let alpha = if is_selected { 0.7 } else { 0.5 };
                
                let label_bg_rect = RoundedRect::from_rect(label_rect, RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0));
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(alpha)),
                    None,
                    &label_bg_rect.to_path(0.1),
                );
            }

            // 2. Draw Icon
            // Try to get thumbnail first, fall back to icon
            let mut use_thumbnail = false;
            let thumb_size = 48; 
            
            if let Some(thumbnail_path) = self.thumbnail_provider.get_thumbnail(entry, thumb_size) {
                if let Ok(Some(cached_thumb)) = self.thumbnail_cache.load_or_get(&thumbnail_path, thumb_size) {
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
            
            if !use_thumbnail {
                // Request thumbnail generation if supported
                if self.thumbnail_provider.is_supported(entry) {
                    let mut pending = self.pending_thumbnails.lock().unwrap();
                    if !pending.contains(&entry.path) {
                        if let Ok(()) = self.thumbnail_provider.request_thumbnail(entry, thumb_size) {
                            pending.insert(entry.path.clone());
                        }
                    }
                }
                
                // Get icon for this entry
                let cache_key = (entry.path.clone(), thumb_size);
                let cached_icon = {
                    let mut cache = self.icon_cache.lock().unwrap();
                    if let Some(icon) = cache.get(&cache_key) {
                        icon.clone()
                    } else {
                        let icon = self.icon_registry.get_file_icon(entry, thumb_size);
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
                    // Fallback
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

            // 3. Draw Icon Overlay (Selection/Hover)
            if is_selected || is_hovered {
                let color_prop = if is_selected {
                    nptk_theme::properties::ThemeProperty::ColorBackgroundSelected
                } else {
                    nptk_theme::properties::ThemeProperty::ColorMenuHovered
                };
                
                let color = theme
                    .get_property(self.widget_id(), &color_prop)
                    .or_else(|| theme.get_default_property(&color_prop))
                    .unwrap_or_else(|| if is_selected { Color::from_rgb8(100, 150, 255) } else { Color::from_rgb8(240, 240, 240) });
                
                let alpha = if is_selected { 0.5 } else { 0.3 };
                
                let icon_overlay_rect = RoundedRect::from_rect(icon_rect, RoundedRectRadii::new(3.0, 3.0, 3.0, 3.0));
                
                graphics.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(color.with_alpha(alpha)),
                    None,
                    &icon_overlay_rect.to_path(0.1),
                );
            }
            
            // 4. Draw Label Text
            let text_color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                .unwrap_or(Color::BLACK);
            
            // Use label_rect to position text (reverse padding)
            let text_x = label_rect.x0 + 4.0; // label_padding_x
            let text_y = label_rect.y0 + 2.0; // label_padding_y
            let max_text_width = cell_width - (32.0 + 8.0 * 2.0 + 10.0); // Re-calculate or pass it? Re-calc is cheap.
            
            let transform = Affine::translate((text_x, text_y));
            
            self.text_render_context.render_text_with_max_lines(
                &mut info.font_context,
                graphics,
                &entry.name,
                None,
                font_size,
                Brush::Solid(text_color),
                transform,
                true,
                Some(max_text_width as f32),
                Some(2), // Max 2 lines
                false, // Left align
            );
        }
    }
}
