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
use nptk_core::vg::kurbo::{Affine, Rect, Shape, Vec2};
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
mod thumbnail_cache;
use thumbnail_cache::ThumbnailImageCache;

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
            selected_path.clone(),
            current_path.clone(),
            fs_model.clone(),
            icon_registry.clone(),
            thumbnail_provider.clone(),
            thumbnail_event_rx,
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
    icon_registry: Arc<IconRegistry>,
    thumbnail_provider: Arc<dyn ThumbnailProvider>,
    
    item_height: f32,
    text_render_context: TextRenderContext,
    thumbnail_size: u32,
    
    // Input state
    last_click_time: Option<Instant>,
    last_click_index: Option<usize>,
    
    // Icon cache per entry (to avoid repeated lookups)
    icon_cache: Arc<Mutex<std::collections::HashMap<(PathBuf, u32), Option<nptk_services::icon::CachedIcon>>>>,
    
    // Thumbnail cache for decoded images
    thumbnail_cache: Arc<ThumbnailImageCache>,
    
    // Track pending thumbnail requests to avoid duplicate requests
    pending_thumbnails: Arc<Mutex<HashSet<PathBuf>>>,
    
    // Thumbnail event receiver
    thumbnail_event_rx: Arc<Mutex<tokio::sync::broadcast::Receiver<ThumbnailEvent>>>,
}

impl FileListContent {
    fn new(
        entries: StateSignal<Vec<FileEntry>>,
        selected_path: StateSignal<Option<PathBuf>>,
        current_path: StateSignal<PathBuf>,
        fs_model: Arc<FileSystemModel>,
        icon_registry: Arc<IconRegistry>,
        thumbnail_provider: Arc<dyn ThumbnailProvider>,
        thumbnail_event_rx: tokio::sync::broadcast::Receiver<ThumbnailEvent>,
    ) -> Self {
        Self {

            entries,
            selected_path,
            current_path,
            fs_model,
            icon_registry,
            thumbnail_provider,
            item_height: 30.0,
            text_render_context: TextRenderContext::new(),
            thumbnail_size: 128, // Default thumbnail size
            last_click_time: None,
            last_click_index: None,
            icon_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
            thumbnail_cache: Arc::new(ThumbnailImageCache::default()),
            pending_thumbnails: Arc::new(Mutex::new(HashSet::new())),
            thumbnail_event_rx: Arc::new(Mutex::new(thumbnail_event_rx)),
        }
    }
    
    /// Set the thumbnail size for this file list.
    pub fn with_thumbnail_size(mut self, size: u32) -> Self {
        self.thumbnail_size = size;
        self
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
        
        // Use theme background with proper fallback chain
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
            if is_hovered && Some(&entry.path) != selected.as_ref() {
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
            if Some(&entry.path) == selected.as_ref() {
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
                // Try to load thumbnail from cache
                if let Ok(Some(cached_thumb)) = self.thumbnail_cache.load_or_get(&thumbnail_path, self.thumbnail_size) {
                    // Render thumbnail
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
                // Request thumbnail generation if supported and not already pending
                if self.thumbnail_provider.is_supported(entry) {
                    let mut pending = self.pending_thumbnails.lock().unwrap();
                    if !pending.contains(&entry.path) {
                        if let Ok(()) = self.thumbnail_provider.request_thumbnail(entry, self.thumbnail_size) {
                            pending.insert(entry.path.clone());
                        }
                    }
                }
                
                // Get icon for this entry
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
                    // Render icon using FileIcon widget approach
                    // For now, we'll render directly here since we're in the render method
                    // In a more sophisticated implementation, we could use child widgets
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
                            // Fallback to themed colored rectangle
                            // Use text color with reduced opacity for icon fallback
                            let icon_color = theme
                                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                                .unwrap_or(Color::from_rgb8(150, 150, 150));
                            
                            let fallback_color = if entry.file_type == FileType::Directory {
                                // Slightly different color for directories
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
                    // Fallback to themed colored rectangle if no icon found
                    let icon_color = theme
                        .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                        .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
                        .unwrap_or(Color::from_rgb8(150, 150, 150));
                    
                    let fallback_color = if entry.file_type == FileType::Directory {
                        // Slightly different color for directories
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
            
            // Draw text with proper theme fallback
            let text_color = theme
                .get_property(self.widget_id(), &nptk_theme::properties::ThemeProperty::ColorText)
                .or_else(|| theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorText))
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
