use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use humansize::{format_size, BINARY};
use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::font_ctx::FontContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::menu::{ContextMenu, ContextMenuGroup, ContextMenuItem};
use nptk_core::signal::{state::StateSignal, MaybeSignal, Signal};
use nptk_core::text_render::TextRenderContext;
use nptk_core::vg::kurbo::{
    Affine, Point, Rect, RoundedRect, RoundedRectRadii, Shape, Stroke, Vec2,
};
use nptk_core::vg::peniko::{Brush, Color, Fill};
use nptk_core::vgi::Graphics;
use nptk_core::widget::{BoxedWidget, Widget, WidgetLayoutExt};
use nptk_core::window::{ElementState, MouseButton};
use nptk_services::filesystem::entry::{FileEntry, FileType};
use nptk_services::filesystem::model::{FileSystemEvent, FileSystemModel};
use nptk_services::icon::IconRegistry;
use nptk_services::thumbnail::events::ThumbnailEvent;
use nptk_services::thumbnail::{ThumbnailProvider, ThumbnailifyProvider};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::collections::HashSet;
use std::fs;
use tokio::sync::broadcast;

mod view_compact;
mod view_icon;
mod view_list;

use crate::scroll_container::{ScrollContainer, ScrollDirection};
use crate::tabs_container::{TabItem, TabsContainer};
use nptk_services::filesystem::{mime_registry::MimeRegistry, MimeDetector};
use nptk_services::thumbnail::ThumbnailImageCache;
use std::path::{Path, PathBuf};
use std::process::Command;

/// View mode for the file list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        let icon_registry =
            Arc::new(IconRegistry::new().unwrap_or_else(|_| IconRegistry::default()));

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
            }
            .into(),
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
                    },
                    _ => {
                        // For other events, we might want to refresh if they affect current path
                        // But for now, let's just rely on DirectoryLoaded
                    },
                }
            }
        }

        // Update child (ScrollContainer)
        if !layout.children.is_empty() {
            update |= self
                .scroll_container
                .update(&layout.children[0], context.clone(), info);
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
            self.scroll_container
                .render(graphics, theme, &layout.children[0], info, context);
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
    icon_cache: Arc<
        Mutex<std::collections::HashMap<(PathBuf, u32), Option<nptk_services::icon::CachedIcon>>>,
    >,

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

    // Layout cache to avoid expensive recalculations on every frame
    // Key: (path, view_mode, cell_width/icon_size)
    // Value: (icon_rect, label_rect, display_text, max_text_width)
    layout_cache: std::collections::HashMap<
        (PathBuf, FileListViewMode, u32, bool),
        (Rect, Rect, String, f32),
    >,
    cache_invalidated: bool,
    last_layout_width: f32,

    // Icon view constants
    icon_view_padding: f32,
    icon_view_spacing: f32,
    icon_view_text_height: f32,

    // SVG Scene cache to avoid re-parsing SVGs every frame
    // Key: SVG source string (or hash of it)
    // Value: (Scene, width, height)
    svg_scene_cache: std::collections::HashMap<String, (nptk_core::vg::Scene, f64, f64)>,
    mime_registry: MimeRegistry,
    pending_action: Arc<Mutex<Option<PendingAction>>>,
    last_cursor: Option<Point>,
}

#[derive(Clone)]
struct PendingAction {
    paths: Vec<PathBuf>,
    app_id: Option<String>,
    properties: bool,
}

struct PropertiesData {
    title: String,
    icon_label: String,
    rows: Vec<(String, String)>,
    paths: Vec<PathBuf>,
}

struct PropertiesContent {
    data: PropertiesData,
    text_ctx: TextRenderContext,
    icon_registry: Arc<IconRegistry>,
    thumbnail_provider: Arc<dyn ThumbnailProvider>,
    thumbnail_cache: Arc<ThumbnailImageCache>,
    icon_cache: Arc<
        Mutex<std::collections::HashMap<(PathBuf, u32), Option<nptk_services::icon::CachedIcon>>>,
    >,
    svg_scene_cache:
        Arc<Mutex<std::collections::HashMap<String, (nptk_core::vg::Scene, f64, f64)>>>,
    thumbnail_size: u32,
}

impl PropertiesContent {
    fn new(
        data: PropertiesData,
        icon_registry: Arc<IconRegistry>,
        thumbnail_provider: Arc<dyn ThumbnailProvider>,
        thumbnail_cache: Arc<ThumbnailImageCache>,
        icon_cache: Arc<
            Mutex<
                std::collections::HashMap<(PathBuf, u32), Option<nptk_services::icon::CachedIcon>>,
            >,
        >,
        svg_scene_cache: Arc<
            Mutex<std::collections::HashMap<String, (nptk_core::vg::Scene, f64, f64)>>,
        >,
    ) -> Self {
        Self {
            data,
            text_ctx: TextRenderContext::new(),
            icon_registry,
            thumbnail_provider,
            thumbnail_cache,
            icon_cache,
            svg_scene_cache,
            thumbnail_size: 128,
        }
    }
}

impl Widget for PropertiesContent {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "PropertiesContent")
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: LayoutStyle {
                // Fill the available popup space so text is visible.
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            },
            children: vec![],
        }
    }

    fn update(
        &mut self,
        _layout: &LayoutNode,
        _context: AppContext,
        _info: &mut AppInfo,
    ) -> Update {
        Update::empty()
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        info: &mut AppInfo,
        _context: AppContext,
    ) {
        let bg = theme.window_background();
        let rect = Rect::new(
            layout.layout.location.x as f64,
            layout.layout.location.y as f64,
            (layout.layout.location.x + layout.layout.size.width) as f64,
            (layout.layout.location.y + layout.layout.size.height) as f64,
        );
        graphics.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(bg),
            None,
            &rect.to_path(4.0),
        );

        let widget_id = self.widget_id();
        // Try to get ColorText for PropertiesContent, fallback to Text widget's ColorText
        let text_color = theme
            .get_property(
                widget_id.clone(),
                &nptk_theme::properties::ThemeProperty::ColorText,
            )
            .or_else(|| {
                // Fallback to Text widget's ColorText property
                let text_widget_id = nptk_theme::id::WidgetId::new("nptk-widgets", "Text");
                theme.get_property(
                    text_widget_id,
                    &nptk_theme::properties::ThemeProperty::ColorText,
                )
            })
            .or_else(|| {
                // Fallback to Text widget's Color property
                let text_widget_id = nptk_theme::id::WidgetId::new("nptk-widgets", "Text");
                theme.get_property(
                    text_widget_id,
                    &nptk_theme::properties::ThemeProperty::Color,
                )
            })
            .unwrap_or_else(|| Color::BLACK);

        // Try to get ColorTextDisabled for PropertiesContent, fallback to Text widget's ColorTextDisabled
        let label_color = theme
            .get_property(
                widget_id.clone(),
                &nptk_theme::properties::ThemeProperty::ColorTextDisabled,
            )
            .or_else(|| {
                // Fallback to Text widget's ColorTextDisabled property
                let text_widget_id = nptk_theme::id::WidgetId::new("nptk-widgets", "Text");
                theme.get_property(
                    text_widget_id,
                    &nptk_theme::properties::ThemeProperty::ColorTextDisabled,
                )
            })
            .or_else(|| {
                // Fallback to ColorDisabled
                theme.get_default_property(&nptk_theme::properties::ThemeProperty::ColorDisabled)
            })
            .unwrap_or_else(|| Color::from_rgb8(140, 140, 140));

        let padding = 12.0;
        let icon_size = 48.0;
        let icon_rect = Rect::new(
            rect.x0 + padding,
            rect.y0 + padding,
            rect.x0 + padding + icon_size,
            rect.y0 + padding + icon_size,
        );

        // Try to render icon/thumbnail, fallback to text label
        let mut icon_rendered = false;

        // For multiple files, try multi-file icon
        if self.data.paths.len() > 1 {
            // Try document-multiple or folder-multiple icons
            let icon_names = ["document-multiple", "folder-multiple", "document", "folder"];
            for icon_name in &icon_names {
                if let Some(icon) = self.icon_registry.get_icon(icon_name, icon_size as u32) {
                    let icon_x = icon_rect.x0;
                    let icon_y = icon_rect.y0;
                    let icon_size_f64 = icon_rect.width().min(icon_rect.height());

                    match icon {
                        nptk_services::icon::CachedIcon::Image {
                            data,
                            width,
                            height,
                        } => {
                            use nptk_core::vg::peniko::{
                                Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
                            };
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
                                icon_rendered = true;
                                break;
                            }
                        },
                        nptk_services::icon::CachedIcon::Svg(svg_source) => {
                            // Check SVG scene cache first
                            let cached_scene = {
                                let cache = self.svg_scene_cache.lock().unwrap();
                                cache.get(svg_source.as_str()).cloned()
                            };
                            let (scene, svg_width, svg_height) = if let Some((scene, w, h)) =
                                cached_scene
                            {
                                (scene, w, h)
                            } else {
                                // Cache miss - parse and render SVG
                                use vello_svg::usvg::{
                                    ImageRendering, Options, ShapeRendering, TextRendering, Tree,
                                };
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
                                    let w = svg_size.width() as f64;
                                    let h = svg_size.height() as f64;
                                    {
                                        let mut cache = self.svg_scene_cache.lock().unwrap();
                                        cache.insert(
                                            svg_source.as_str().to_string(),
                                            (scene.clone(), w, h),
                                        );
                                    }
                                    (scene, w, h)
                                } else {
                                    (nptk_core::vg::Scene::new(), 1.0, 1.0)
                                }
                            };

                            let scale_x = icon_size_f64 / svg_width;
                            let scale_y = icon_size_f64 / svg_height;
                            let scale = scale_x.min(scale_y);
                            let transform = Affine::scale_non_uniform(scale, scale)
                                .then_translate(Vec2::new(icon_x, icon_y));
                            graphics.append(&scene, Some(transform));
                            icon_rendered = true;
                            break;
                        },
                        nptk_services::icon::CachedIcon::Path(_) => {
                            // Path icons are rendered as fallback below
                        },
                    }
                }
            }
        }

        // For single file, try thumbnail first, then icon
        if !icon_rendered && self.data.paths.len() == 1 {
            let path = &self.data.paths[0];

            // Create FileEntry from path
            let entry = if let Ok(metadata) = fs::metadata(path) {
                let file_type = if metadata.is_dir() {
                    FileType::Directory
                } else if metadata.is_file() {
                    FileType::File
                } else if metadata.file_type().is_symlink() {
                    FileType::Symlink
                } else {
                    FileType::Other
                };

                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let mime_type = if file_type == FileType::File {
                    MimeDetector::detect_mime_type(path)
                } else {
                    None
                };

                use nptk_services::filesystem::entry::FileMetadata;
                if let Ok(modified) = metadata.modified() {
                    let file_metadata = FileMetadata {
                        size: metadata.len(),
                        modified,
                        created: metadata.created().ok(),
                        permissions: 0,
                        mime_type,
                        is_hidden: name.starts_with('.'),
                    };

                    Some(FileEntry::new(
                        path.clone(),
                        name,
                        file_type,
                        file_metadata,
                        path.parent().map(|p| p.to_path_buf()),
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            // Try thumbnail first
            if let Some(entry) = entry {
                if let Some(thumbnail_path) = self
                    .thumbnail_provider
                    .get_thumbnail(&entry, self.thumbnail_size)
                {
                    if let Ok(Some(cached_thumb)) = self
                        .thumbnail_cache
                        .load_or_get(&thumbnail_path, self.thumbnail_size)
                    {
                        use nptk_core::vg::peniko::{
                            Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
                        };
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
                            icon_rendered = true;
                        }
                    }
                }

                // If no thumbnail, try icon
                if !icon_rendered {
                    let cache_key = (path.clone(), icon_size as u32);
                    let cached_icon = {
                        let mut cache = self.icon_cache.lock().unwrap();
                        if let Some(icon) = cache.get(&cache_key) {
                            icon.clone()
                        } else {
                            let icon = self.icon_registry.get_file_icon(&entry, icon_size as u32);
                            cache.insert(cache_key.clone(), icon.clone());
                            icon
                        }
                    };

                    if let Some(icon) = cached_icon {
                        let icon_x = icon_rect.x0;
                        let icon_y = icon_rect.y0;
                        let icon_size_f64 = icon_rect.width().min(icon_rect.height());

                        match icon {
                            nptk_services::icon::CachedIcon::Image {
                                data,
                                width,
                                height,
                            } => {
                                use nptk_core::vg::peniko::{
                                    Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
                                };
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
                                    icon_rendered = true;
                                }
                            },
                            nptk_services::icon::CachedIcon::Svg(svg_source) => {
                                // Check SVG scene cache first
                                let cached_scene = {
                                    let cache = self.svg_scene_cache.lock().unwrap();
                                    cache.get(svg_source.as_str()).cloned()
                                };
                                let (scene, svg_width, svg_height) = if let Some((scene, w, h)) =
                                    cached_scene
                                {
                                    (scene, w, h)
                                } else {
                                    // Cache miss - parse and render SVG
                                    use vello_svg::usvg::{
                                        ImageRendering, Options, ShapeRendering, TextRendering,
                                        Tree,
                                    };
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
                                        let w = svg_size.width() as f64;
                                        let h = svg_size.height() as f64;
                                        {
                                            let mut cache = self.svg_scene_cache.lock().unwrap();
                                            cache.insert(
                                                svg_source.as_str().to_string(),
                                                (scene.clone(), w, h),
                                            );
                                        }
                                        (scene, w, h)
                                    } else {
                                        (nptk_core::vg::Scene::new(), 1.0, 1.0)
                                    }
                                };

                                let scale_x = icon_size_f64 / svg_width;
                                let scale_y = icon_size_f64 / svg_height;
                                let scale = scale_x.min(scale_y);
                                let transform = Affine::scale_non_uniform(scale, scale)
                                    .then_translate(Vec2::new(icon_x, icon_y));
                                graphics.append(&scene, Some(transform));
                                icon_rendered = true;
                            },
                            nptk_services::icon::CachedIcon::Path(_) => {
                                // Path icons are rendered as fallback below
                            },
                        }
                    }
                }
            }
        }

        // Fallback: render placeholder with text label
        if !icon_rendered {
            // Icon placeholder
            graphics.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(Color::from_rgb8(200, 200, 200)),
                None,
                &icon_rect.to_path(4.0),
            );
            // Icon label
            self.text_ctx.render_text(
                &mut info.font_context,
                graphics,
                &self.data.icon_label,
                None,
                12.0,
                Brush::Solid(Color::from_rgb8(60, 60, 60)),
                Affine::translate((icon_rect.x0 + 6.0, icon_rect.y0 + icon_size / 2.0 - 6.0)),
                true,
                Some((icon_size - 12.0) as f32),
            );
        }

        // Title
        self.text_ctx.render_text(
            &mut info.font_context,
            graphics,
            &self.data.title,
            None,
            16.0,
            Brush::Solid(text_color),
            Affine::translate((icon_rect.x1 + 10.0, icon_rect.y0 + 4.0)),
            true,
            Some(
                (rect.width() as f32 - (icon_rect.width() as f32) - 3.0 * padding as f32).max(80.0),
            ),
        );

        let mut y = icon_rect.y1 + 12.0;
        let label_width = 110.0;
        let value_x = rect.x0 + padding + label_width + 8.0;

        for (label, value) in &self.data.rows {
            self.text_ctx.render_text(
                &mut info.font_context,
                graphics,
                &format!("{}:", label),
                None,
                13.0,
                Brush::Solid(text_color).with_alpha(0.95),
                Affine::translate((rect.x0 + padding, y)),
                true,
                Some(label_width as f32),
            );
            self.text_ctx.render_text(
                &mut info.font_context,
                graphics,
                value,
                None,
                13.0,
                Brush::Solid(text_color),
                Affine::translate((value_x, y)),
                true,
                Some((rect.width() as f32 - value_x as f32 - padding as f32).max(60.0)),
            );
            y += 20.0;
        }
    }
}

impl FileListContent {
    /// Build properties widget wrapped in a tab container.
    fn build_properties_widget(
        data: PropertiesData,
        icon_registry: Arc<IconRegistry>,
        thumbnail_provider: Arc<dyn ThumbnailProvider>,
        thumbnail_cache: Arc<ThumbnailImageCache>,
        icon_cache: Arc<
            Mutex<
                std::collections::HashMap<(PathBuf, u32), Option<nptk_services::icon::CachedIcon>>,
            >,
        >,
        svg_scene_cache: Arc<
            Mutex<std::collections::HashMap<String, (nptk_core::vg::Scene, f64, f64)>>,
        >,
    ) -> BoxedWidget {
        let content = PropertiesContent::new(
            data,
            icon_registry,
            thumbnail_provider,
            thumbnail_cache,
            icon_cache,
            svg_scene_cache,
        );
        let tab = TabItem::new("general", "General", content);
        let tabs = TabsContainer::new()
            .with_layout_style(LayoutStyle {
                size: Vector2::new(Dimension::percent(1.0), Dimension::percent(1.0)),
                ..Default::default()
            })
            .with_tab(tab);
        Box::new(tabs)
    }

    fn format_system_time(time: std::time::SystemTime) -> String {
        let dt: DateTime<Local> = time.into();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

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

            layout_cache: std::collections::HashMap::new(),
            cache_invalidated: false,
            last_layout_width: 1000.0,

            icon_view_padding: 2.0,      // padding around the icons
            icon_view_spacing: 22.0,     // spacing between icons
            icon_view_text_height: 50.0, // Increased to accommodate 2-3 lines of wrapped text

            svg_scene_cache: std::collections::HashMap::new(),
            mime_registry: MimeRegistry::load_default(),
            pending_action: Arc::new(Mutex::new(None)),
            last_cursor: None,
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

    /// Open a file path using MIME resolution (no self captures; suitable for menu callbacks).
    fn launch_path(registry: MimeRegistry, path: PathBuf) {
        // Detect MIME type with fallback to xdg-mime filetype.
        let mime = MimeDetector::detect_mime_type(&path).or_else(|| Self::xdg_mime_filetype(&path));
        let Some(mime) = mime else {
            log::warn!("Could not detect MIME type for {:?}", path);
            return;
        };

        // Resolve default app, otherwise first handler, otherwise xdg-open fallback.
        let app = registry.resolve(&mime).or_else(|| {
            let mut handlers = registry.list_handlers(&mime);
            handlers.into_iter().next()
        });

        if let Some(app_id) = app {
            if let Err(err) = registry.launch(&app_id, &path) {
                log::warn!("Failed to launch app '{}': {}", app_id, err);
            }
            return;
        }

        // Fallback: xdg-open
        match Command::new("xdg-open").arg(path).spawn() {
            Ok(_) => {},
            Err(err) => {
                log::warn!(
                    "No application found for MIME {} and xdg-open failed: {}",
                    mime,
                    err
                );
            },
        }
    }

    /// Try to obtain a user-visible app name for a path's MIME type.
    fn open_label_for_path(&self, path: &Path) -> String {
        // Directories keep generic label.
        if path.is_dir() {
            return "Open".to_string();
        }

        let mime = MimeDetector::detect_mime_type(path).or_else(|| Self::xdg_mime_filetype(path));
        let Some(mime) = mime else {
            return "Open".to_string();
        };

        // Try to resolve app name with the detected MIME type and alternatives
        let mime_variants = Self::get_mime_variants(&mime);
        for variant in &mime_variants {
            // 1) Use registry default
            if let Some((_, name)) = self.mime_registry.resolve_with_name(variant) {
                return format!("Open with {}", name);
            }

            // 2) Try first handler and resolve its name
            let handlers = self.mime_registry.list_handlers(variant);
            if let Some(app_id) = handlers.into_iter().next() {
                let name = self.display_name_for_appid(&app_id);
                return format!("Open with {}", name);
            }

            // 3) Ask xdg-mime for default and resolve name
            if let Some(app_id) = Self::xdg_default_for_mime(variant) {
                let name = self.display_name_for_appid(&app_id);
                return format!("Open with {}", name);
            }
        }

        "Open".to_string()
    }

    /// Get alternative MIME type variants to try when resolving apps.
    /// This helps when the detected MIME type doesn't match system registrations.
    fn get_mime_variants(mime: &str) -> Vec<String> {
        let mut variants = vec![mime.to_string()];

        // Map non-standard MIME types to standard alternatives
        match mime {
            "text/x-toml" => {
                variants.push("application/toml".to_string());
                variants.push("text/plain".to_string());
            },
            "application/toml" => {
                // TOML files are often handled by text editors via text/plain
                variants.push("text/plain".to_string());
            },
            "text/x-rust" => {
                variants.push("text/plain".to_string());
            },
            mime if mime.starts_with("text/") => {
                // For any text/* type, also try text/plain as fallback
                if mime != "text/plain" {
                    variants.push("text/plain".to_string());
                }
            },
            // For application/* types that are likely text-based, try text/plain
            mime if mime.starts_with("application/")
                && (mime.contains("json")
                    || mime.contains("xml")
                    || mime.contains("yaml")
                    || mime.contains("toml")
                    || mime.contains("markdown")) =>
            {
                variants.push("text/plain".to_string());
            },
            _ => {},
        }

        variants
    }

    /// Build "Open With" submenu items for a path and selection.
    fn build_open_with_items(&self, path: &Path, selection: Vec<PathBuf>) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();

        let mime = MimeDetector::detect_mime_type(path).or_else(|| Self::xdg_mime_filetype(path));
        let Some(mime) = mime else {
            return items;
        };

        let variants = Self::get_mime_variants(&mime);
        let mut seen: HashSet<String> = HashSet::new();
        let mut handlers: Vec<String> = Vec::new();

        for variant in variants {
            if let Some(default_id) = self.mime_registry.resolve(&variant) {
                if seen.insert(default_id.clone()) {
                    handlers.push(default_id);
                }
            }
            for app_id in self.mime_registry.list_handlers(&variant) {
                if seen.insert(app_id.clone()) {
                    handlers.push(app_id);
                }
            }
            if let Some(app_id) = Self::xdg_default_for_mime(&variant) {
                if seen.insert(app_id.clone()) {
                    handlers.push(app_id);
                }
            }
        }

        for app_id in handlers {
            let label = self.display_name_for_appid(&app_id);
            let pending = self.pending_action.clone();
            let paths_for_action = selection.clone();
            let app_id_cloned = app_id.clone();
            items.push(ContextMenuItem::Action {
                label,
                action: Arc::new(move || {
                    if let Ok(mut pending_lock) = pending.lock() {
                        *pending_lock = Some(PendingAction {
                            paths: paths_for_action.clone(),
                            app_id: Some(app_id_cloned.clone()),
                            properties: false,
                        });
                    }
                }),
            });
        }

        items
    }

    fn show_properties_popup(&self, paths: &[PathBuf], context: AppContext) {
        if paths.is_empty() {
            return;
        }

        let mut rows: Vec<(String, String)> = Vec::new();
        let mut title = String::new();
        let mut icon_label = String::new();

        if paths.len() == 1 {
            let path = &paths[0];
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("<unnamed>");
            title = name.to_string();
            icon_label = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_uppercase())
                .unwrap_or_else(|| "FILE".to_string());

            let mime_type = MimeDetector::detect_mime_type(path)
                .or_else(|| Self::xdg_mime_filetype(path))
                .unwrap_or_else(|| "unknown".to_string());

            let kind_display = if let Some(description) = self.lookup_mime_description(&mime_type) {
                format!("{} ({})", description, mime_type)
            } else {
                mime_type.clone()
            };
            rows.push(("Kind".to_string(), kind_display));
            rows.push(("Name".to_string(), name.to_string()));

            if let Ok(meta) = fs::metadata(path) {
                let size = if meta.is_dir() {
                    Self::calculate_directory_size(path)
                } else {
                    meta.len()
                };
                rows.push((
                    "Size".to_string(),
                    format_size(size, BINARY) + " (" + size.to_string().as_str() + " bytes)",
                ));
                if let Ok(modified) = meta.modified() {
                    rows.push(("Modified".to_string(), Self::format_system_time(modified)));
                }
                if let Ok(created) = meta.created() {
                    rows.push(("Created".to_string(), Self::format_system_time(created)));
                }
            }

            rows.push((
                "Location".to_string(),
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "".to_string()),
            ));
            rows.push(("Path".to_string(), path.display().to_string()));
        } else {
            let count = paths.len();
            let mut total_size: u64 = 0;
            for p in paths {
                if let Ok(meta) = fs::metadata(p) {
                    let size = if meta.is_dir() {
                        Self::calculate_directory_size(p)
                    } else {
                        meta.len()
                    };
                    total_size = total_size.saturating_add(size);
                }
            }
            title = format!("{} items", count);
            icon_label = "MULTI".to_string();
            rows.push(("Items".to_string(), count.to_string()));
            rows.push(("Total size".to_string(), format_size(total_size, BINARY)));
        }

        let data = PropertiesData {
            title,
            icon_label,
            rows,
            paths: paths.to_vec(),
        };
        // Create a new SVG scene cache for the properties widget
        let svg_scene_cache = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let props_widget = FileListContent::build_properties_widget(
            data,
            self.icon_registry.clone(),
            self.thumbnail_provider.clone(),
            self.thumbnail_cache.clone(),
            self.icon_cache.clone(),
            svg_scene_cache,
        );
        let pos = self
            .last_cursor
            .map(|p| (p.x as i32, p.y as i32))
            .unwrap_or((100, 100));
        context
            .popup_manager
            .create_popup_at(props_widget, "Properties", (360, 260), pos);
    }

    fn xdg_mime_filetype(path: &Path) -> Option<String> {
        let output = Command::new("xdg-mime")
            .args(["query", "filetype", path.to_string_lossy().as_ref()])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mime = stdout.trim();
        if mime.is_empty() {
            None
        } else {
            Some(mime.to_string())
        }
    }

    /// Extract the first <comment> text from a mime-type block, preferring xml:lang="en".
    fn extract_comment(mime_block: &str) -> Option<String> {
        let mut best: Option<String> = None;
        let mut fallback: Option<String> = None;
        let mut search_start = 0;
        while let Some(idx) = mime_block[search_start..].find("<comment") {
            let comment_start = search_start + idx;
            let tag_end = match mime_block[comment_start..].find('>') {
                Some(i) => comment_start + i + 1,
                None => break,
            };
            let end_tag = match mime_block[tag_end..].find("</comment>") {
                Some(i) => tag_end + i,
                None => break,
            };
            let tag_text = &mime_block[comment_start..tag_end];
            let body = mime_block[tag_end..end_tag].trim();
            if body.is_empty() {
                search_start = end_tag + "</comment>".len();
                continue;
            }
            let is_en = tag_text.contains(r#"xml:lang="en""#);
            if is_en {
                best = Some(body.to_string());
                break;
            } else if fallback.is_none() {
                fallback = Some(body.to_string());
            }
            search_start = end_tag + "</comment>".len();
        }
        best.or(fallback)
    }

    /// Generate variant MIME types to try when resolving a description.
    fn mime_description_variants(mime_type: &str) -> Vec<String> {
        let mut variants = Vec::new();
        variants.push(mime_type.to_string());
        if let Some((major, rest)) = mime_type.split_once('/') {
            if let Some(stripped) = rest.strip_prefix("x-") {
                variants.push(format!("{}/{}", major, stripped));
            }
        }
        match mime_type {
            "application/toml" => variants.push("text/x-toml".to_string()),
            "text/x-rust" => variants.push("text/rust".to_string()),
            "application/x-shellscript" => {
                variants.push("text/x-shellscript".to_string());
                variants.push("text/x-sh".to_string());
            },
            "application/zstd" => variants.push("application/x-zstd".to_string()),
            "application/x-rar" => variants.push("application/vnd.rar".to_string()),
            "application/x-iso9660-image" => {
                variants.push("application/x-iso9660-image".to_string())
            },
            "text/x-log" => variants.push("text/plain".to_string()),
            _ => {},
        }
        variants
    }

    /// Try to get MIME description via registry (with variants) and fall back to legacy parsing.
    fn lookup_mime_description(&self, mime_type: &str) -> Option<String> {
        for variant in Self::mime_description_variants(mime_type) {
            if let Some(desc) = self.mime_registry.description(&variant) {
                return Some(desc);
            }
        }
        Self::get_mime_description(mime_type)
    }

    /// Try to get MIME description from /usr/share/mime/{major}/{minor}.xml (exact), then fall back
    /// to scanning /usr/share/mime/packages/*.xml. Returns None if not found.
    fn get_mime_description(mime_type: &str) -> Option<String> {
        for variant in Self::mime_description_variants(mime_type) {
            if let Some(desc) = Self::get_mime_description_single(&variant) {
                return Some(desc);
            }
        }
        None
    }

    /// Get description for a single MIME value.
    fn get_mime_description_single(mime_type: &str) -> Option<String> {
        // 1) Try exact file at /usr/share/mime/{major}/{minor}.xml
        if let Some((major, minor)) = mime_type.split_once('/') {
            let path = Path::new("/usr/share/mime")
                .join(major)
                .join(format!("{minor}.xml"));
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains(&format!(r#"type="{}""#, mime_type)) {
                    if let Some(comment) = Self::extract_comment(&content) {
                        return Some(comment);
                    }
                }
            }
        }

        // 2) Fallback: scan packages XMLs for an exact mime-type match
        let packages_dir = Path::new("/usr/share/mime/packages");
        let entries = match fs::read_dir(packages_dir) {
            Ok(entries) => entries,
            Err(_) => return None,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("xml") {
                continue;
            }
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut search_start = 0;
            while let Some(idx) = content[search_start..].find("<mime-type") {
                let mime_start = search_start + idx;
                let tag_end = match content[mime_start..].find('>') {
                    Some(i) => mime_start + i + 1,
                    None => break,
                };
                let tag_text = &content[mime_start..tag_end];

                // Parse type attribute
                let type_attr = r#"type=""#;
                let type_idx = match tag_text.find(type_attr) {
                    Some(i) => i + type_attr.len(),
                    None => {
                        search_start = tag_end;
                        continue;
                    },
                };
                let rest = &tag_text[type_idx..];
                let end_quote = match rest.find('"') {
                    Some(i) => i,
                    None => {
                        search_start = tag_end;
                        continue;
                    },
                };
                let ty = &rest[..end_quote];
                if ty != mime_type {
                    search_start = tag_end;
                    continue;
                }

                // Find end of this mime-type block
                let end_tag = "</mime-type>";
                let block_end = match content[tag_end..].find(end_tag) {
                    Some(i) => tag_end + i + end_tag.len(),
                    None => {
                        search_start = tag_end;
                        continue;
                    },
                };
                let mime_block = &content[mime_start..block_end];
                if let Some(comment) = Self::extract_comment(mime_block) {
                    return Some(comment);
                }
                search_start = block_end;
            }
        }

        None
    }

    /// Recursively calculate the total size of a directory including all files inside.
    fn calculate_directory_size(path: &Path) -> u64 {
        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return 0,
        };

        if !metadata.is_dir() {
            return metadata.len();
        }

        let mut total_size = 0u64;
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return metadata.len(), // Return directory metadata size on error
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();
            let entry_metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if entry_metadata.is_dir() {
                total_size = total_size.saturating_add(Self::calculate_directory_size(&entry_path));
            } else {
                total_size = total_size.saturating_add(entry_metadata.len());
            }
        }

        total_size
    }

    fn xdg_default_for_mime(mime: &str) -> Option<String> {
        let output = Command::new("xdg-mime")
            .args(["query", "default", mime])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let id = stdout.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    }

    fn display_name_for_appid(&self, app_id: &str) -> String {
        // Use the registry's prettification method which handles all cases
        self.mime_registry.name_or_prettify(app_id)
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
            let (columns, cell_width, cell_height) =
                self.calculate_icon_view_layout(layout_width, icon_size);

            for (i, entry) in entries.iter().enumerate() {
                let (x, y) = self.get_icon_position(i, columns, cell_width, cell_height);
                // We use the full cell rect for intersection to make it easier to select
                let cell_rect = Rect::new(
                    x as f64,
                    y as f64,
                    (x + cell_width) as f64,
                    (y + cell_height) as f64,
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
            let (columns, cell_width, cell_height, spacing) =
                self.calculate_compact_view_layout(layout_width);

            for (i, entry) in entries.iter().enumerate() {
                let col = i % columns;
                let row = i / columns;
                let x = self.icon_view_padding + col as f32 * (cell_width + spacing);
                let y = self.icon_view_padding + row as f32 * (cell_height + spacing);

                let cell_rect = Rect::new(
                    x as f64,
                    y as f64,
                    (x + cell_width) as f64,
                    (y + cell_height) as f64,
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
                    (y + self.item_height) as f64,
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
        let width = self.last_layout_width.max(1.0);

        let height = if view_mode == FileListViewMode::Icon {
            let icon_size = *self.icon_size.get();
            let (columns, _, cell_height) = self.calculate_icon_view_layout(width, icon_size);
            let rows = (count as f32 / columns as f32).ceil();
            (rows * cell_height + self.icon_view_padding * 2.0).max(100.0)
        } else if view_mode == FileListViewMode::Compact {
            let (columns, _, cell_height, spacing) = self.calculate_compact_view_layout(width);
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

        if let Some(cursor) = info.cursor_pos {
            self.last_cursor = Some(Point::new(cursor.x, cursor.y));
        }

        // Track viewport width changes to keep height estimation accurate and invalidate cached layouts.
        let current_width = layout.layout.size.width.max(1.0);
        if (current_width - self.last_layout_width).abs() > f32::EPSILON {
            self.last_layout_width = current_width;
            self.layout_cache.clear();
        }

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
                    },
                    ThumbnailEvent::ThumbnailFailed {
                        entry_path, error, ..
                    } => {
                        log::warn!(
                            "Thumbnail generation failed for {:?}: {}",
                            entry_path,
                            error
                        );
                        let mut pending = self.pending_thumbnails.lock().unwrap();
                        pending.remove(&entry_path);
                    },
                }
            }
        }

        if let Some(cursor) = info.cursor_pos {
            let local_y = cursor.y as f32 - layout.layout.location.y;
            let local_x = cursor.x as f32 - layout.layout.location.x;
            let in_bounds = local_x >= 0.0
                && local_x < layout.layout.size.width
                && local_y >= 0.0
                && local_y < layout.layout.size.height;

            let mut index: Option<usize> = None;
            let mut target_path: Option<PathBuf> = None;
            let mut range_paths: Option<Vec<PathBuf>> = None;
            let mut file_type: Option<FileType> = None;

            if in_bounds {
                let view_mode = *self.view_mode.get();
                index = if view_mode == FileListViewMode::Icon {
                    let icon_size = *self.icon_size.get();
                    let (columns, cell_width, cell_height) =
                        self.calculate_icon_view_layout(layout.layout.size.width, icon_size);
                    let col = (local_x / cell_width).floor() as usize;
                    let row = (local_y / cell_height).floor() as usize;
                    let idx = row * columns + col;

                    let entry_opt = {
                        let entries = self.entries.get();
                        if idx < entries.len() {
                            Some(entries[idx].clone())
                        } else {
                            None
                        }
                    };

                    if let Some(entry) = entry_opt {
                        let cell_x = col as f32 * cell_width;
                        let cell_y = row as f32 * cell_height;
                        let cell_rect = Rect::new(
                            cell_x as f64,
                            cell_y as f64,
                            (cell_x + cell_width) as f64,
                            (cell_y + cell_height) as f64,
                        );
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

                        if (cursor_x >= icon_rect.x0
                            && cursor_x < icon_rect.x1
                            && cursor_y >= icon_rect.y0
                            && cursor_y < icon_rect.y1)
                            || (cursor_x >= label_rect.x0
                                && cursor_x < label_rect.x1
                                && cursor_y >= label_rect.y0
                                && cursor_y < label_rect.y1)
                        {
                            Some(idx)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else if view_mode == FileListViewMode::Compact {
                    let (columns, cell_width, cell_height, spacing) =
                        self.calculate_compact_view_layout(layout.layout.size.width);
                    let col = ((local_x - self.icon_view_padding) / (cell_width + spacing)).floor()
                        as usize;
                    let row = ((local_y - self.icon_view_padding) / (cell_height + spacing)).floor()
                        as usize;

                    let cell_x = self.icon_view_padding + col as f32 * (cell_width + spacing);
                    let cell_y = self.icon_view_padding + row as f32 * (cell_height + spacing);

                    if local_x >= cell_x
                        && local_x < cell_x + cell_width
                        && local_y >= cell_y
                        && local_y < cell_y + cell_height
                    {
                        let idx = row * columns + col;

                        let entry_opt = {
                            let entries = self.entries.get();
                            if idx < entries.len() {
                                Some(entries[idx].clone())
                            } else {
                                None
                            }
                        };

                        if let Some(entry) = entry_opt {
                            let (mut icon_rect, mut label_rect) = self.get_compact_item_layout(
                                &mut info.font_context,
                                &entry,
                                cell_height,
                                cell_width,
                            );
                            icon_rect = icon_rect + Vec2::new(cell_x as f64, cell_y as f64);
                            label_rect = label_rect + Vec2::new(cell_x as f64, cell_y as f64);

                            let cursor_x = local_x as f64;
                            let cursor_y = local_y as f64;

                            if (cursor_x >= icon_rect.x0
                                && cursor_x < icon_rect.x1
                                && cursor_y >= icon_rect.y0
                                && cursor_y < icon_rect.y1)
                                || (cursor_x >= label_rect.x0
                                    && cursor_x < label_rect.x1
                                    && cursor_y >= label_rect.y0
                                    && cursor_y < label_rect.y1)
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
                    let idx = (local_y / self.item_height) as usize;
                    let entries = self.entries.get();
                    if idx < entries.len() {
                        Some(idx)
                    } else {
                        None
                    }
                };

                if let Some(index) = index {
                    let entries = self.entries.get();
                    if index < entries.len() {
                        let entry = &entries[index];
                        target_path = Some(entry.path.clone());
                        file_type = Some(entry.file_type);

                        if info.modifiers.shift_key() {
                            let anchor = self.anchor_index.unwrap_or(0);
                            let start = anchor.min(index);
                            let end = anchor.max(index);
                            range_paths = Some(
                                entries[start..=end]
                                    .iter()
                                    .map(|e| e.path.clone())
                                    .collect::<Vec<_>>(),
                            );
                        }
                    }
                }

                if let Some(target_path) = target_path {
                    let ctrl_pressed = info.modifiers.control_key();
                    let shift_pressed = info.modifiers.shift_key();

                    for (_, btn, el) in &info.buttons {
                        if *btn == MouseButton::Right && *el == ElementState::Pressed {
                            let mut current_selection = self.selected_paths.get().to_vec();
                            if !current_selection.contains(&target_path) {
                                if ctrl_pressed {
                                    current_selection.push(target_path.clone());
                                } else {
                                    current_selection = vec![target_path.clone()];
                                }
                                self.selected_paths.set(current_selection.clone());
                                update.insert(Update::DRAW);
                            }

                            let pending = self.pending_action.clone();
                            let paths_for_action = current_selection.clone();
                            let paths_for_open = paths_for_action.clone();

                            let open_label = self.open_label_for_path(&target_path);

                            let open_with_items =
                                self.build_open_with_items(&target_path, paths_for_action.clone());

                            let mut core_items = vec![ContextMenuItem::Action {
                                label: open_label,
                                action: Arc::new(move || {
                                    if let Ok(mut pending_lock) = pending.lock() {
                                        *pending_lock = Some(PendingAction {
                                            paths: paths_for_open.clone(),
                                            app_id: None,
                                            properties: false,
                                        });
                                    }
                                }),
                            }];
                            if !open_with_items.is_empty() {
                                core_items.push(ContextMenuItem::SubMenu {
                                    label: "Open With".to_string(),
                                    items: open_with_items,
                                });
                            }
                            core_items.push(ContextMenuItem::Action {
                                label: "Delete".to_string(),
                                action: Arc::new(|| {
                                    println!("Delete");
                                }),
                            });
                            let pending_props = self.pending_action.clone();
                            let props_paths = paths_for_action.clone();
                            core_items.push(ContextMenuItem::Action {
                                label: "Properties".to_string(),
                                action: Arc::new(move || {
                                    if let Ok(mut pending_lock) = pending_props.lock() {
                                        *pending_lock = Some(PendingAction {
                                            paths: props_paths.clone(),
                                            app_id: None,
                                            properties: true,
                                        });
                                    }
                                }),
                            });

                            // Placeholder groups for future integrations.
                            let sharing_items = vec![ContextMenuItem::Action {
                                label: "Share (placeholder)".to_string(),
                                action: Arc::new(|| {}),
                            }];
                            let extensions_items = vec![ContextMenuItem::Action {
                                label: "Extensions (placeholder)".to_string(),
                                action: Arc::new(|| {}),
                            }];
                            let view_items = vec![ContextMenuItem::Action {
                                label: "View options (placeholder)".to_string(),
                                action: Arc::new(|| {}),
                            }];

                            let menu = ContextMenu {
                                items: Vec::new(),
                                groups: Some(vec![
                                    ContextMenuGroup { items: core_items },
                                    ContextMenuGroup {
                                        items: sharing_items,
                                    },
                                    ContextMenuGroup {
                                        items: extensions_items,
                                    },
                                    ContextMenuGroup { items: view_items },
                                ]),
                            };
                            if let Some(cursor_pos) = info.cursor_pos {
                                let cursor = Point::new(cursor_pos.x, cursor_pos.y);
                                context.menu_manager.show_context_menu(menu, cursor);
                                update.insert(Update::DRAW);
                            }
                        }

                        if *btn == MouseButton::Left && *el == ElementState::Pressed {
                            let mut selected = self.selected_paths.get().clone();
                            let is_currently_selected = selected.contains(&target_path);

                            if let Some(range_paths) = &range_paths {
                                if ctrl_pressed {
                                    let mut selected_set: HashSet<PathBuf> =
                                        selected.iter().cloned().collect();
                                    for path in range_paths {
                                        selected_set.insert(path.clone());
                                    }
                                    selected = selected_set.into_iter().collect();
                                } else {
                                    selected = range_paths.clone();
                                }
                            } else if ctrl_pressed {
                                if is_currently_selected {
                                    selected.retain(|p| p != &target_path);
                                } else {
                                    selected.push(target_path.clone());
                                }
                                self.anchor_index = Some(index.unwrap_or(0));
                            } else {
                                selected = vec![target_path.clone()];
                                self.anchor_index = Some(index.unwrap_or(0));
                            }

                            self.selected_paths.set(selected);
                            update.insert(Update::DRAW);

                            let now = Instant::now();
                            if let Some(last_time) = self.last_click_time {
                                if let Some(last_index) = self.last_click_index {
                                    if Some(last_index) == index
                                        && now.duration_since(last_time)
                                            < Duration::from_millis(500)
                                    {
                                        if let Some(ftype) = file_type {
                                            if ftype == FileType::Directory {
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
                            self.last_click_index = index;
                        }
                    }
                } else {
                    for (_, btn, el) in &info.buttons {
                        if *btn == MouseButton::Left && *el == ElementState::Pressed {
                            self.drag_start = Some(Point::new(local_x as f64, local_y as f64));
                            self.current_drag_pos =
                                Some(Point::new(local_x as f64, local_y as f64));
                            self.is_dragging = false;

                            if !info.modifiers.control_key() {
                                self.selected_paths.set(Vec::new());
                                update.insert(Update::DRAW);
                            }
                        }
                    }
                }
            }

            // Drag handling outside bounds to keep tracking when cursor leaves the window.
            if let Some(start_pos) = self.drag_start {
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
                    let current_pos = Point::new(local_x as f64, local_y as f64);
                    self.current_drag_pos = Some(current_pos);

                    if !self.is_dragging {
                        let dx = current_pos.x - start_pos.x;
                        let dy = current_pos.y - start_pos.y;
                        if dx.abs() > 5.0 || dy.abs() > 5.0 {
                            self.is_dragging = true;
                        }
                    }

                    if self.is_dragging {
                        let min_x = start_pos.x.min(current_pos.x);
                        let min_y = start_pos.y.min(current_pos.y);
                        let max_x = start_pos.x.max(current_pos.x);
                        let max_y = start_pos.y.max(current_pos.y);

                        let selection_rect = Rect::new(min_x, min_y, max_x, max_y);

                        self.update_drag_selection(
                            selection_rect,
                            info.modifiers.control_key(),
                            layout.layout.size.width,
                        );
                        update.insert(Update::DRAW);
                    }
                }
            }
        }

        // Process any pending action set by context menu callbacks.
        if let Ok(mut pending) = self.pending_action.lock() {
            if let Some(action) = pending.take() {
                if let Some(app_id) = action.app_id {
                    for path in action.paths.iter() {
                        if let Err(err) = self.mime_registry.launch(&app_id, path) {
                            log::warn!(
                                "Failed to launch {} with {}: {}",
                                path.display(),
                                app_id,
                                err
                            );
                        }
                    }
                } else if action.properties {
                    self.show_properties_popup(&action.paths, context);
                } else {
                    if action.paths.len() == 1 {
                        let path = &action.paths[0];
                        if path.is_dir() {
                            self.current_path.set(path.clone());
                            let _ = self.fs_model.refresh(path);
                            self.selected_paths.set(Vec::new());
                            update.insert(Update::LAYOUT | Update::DRAW);
                        } else {
                            FileListContent::launch_path(self.mime_registry.clone(), path.clone());
                        }
                    } else {
                        // Multi-selection: launch all files, skip directories.
                        for path in action.paths.iter() {
                            if path.is_dir() {
                                continue;
                            }
                            FileListContent::launch_path(self.mime_registry.clone(), path.clone());
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
        // DEBUG: Log render calls to investigate frequency
        // use std::sync::atomic::{AtomicU64, Ordering};
        // static RENDER_COUNTER: AtomicU64 = AtomicU64::new(0);
        // let count = RENDER_COUNTER.fetch_add(1, Ordering::Relaxed);
        // if count % 60 == 0 {
        //     // println!("FileList render called {} times", count);
        // }

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
                    .get_property(
                        self.widget_id(),
                        &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected,
                    )
                    .or_else(|| {
                        theme.get_default_property(
                            &nptk_theme::properties::ThemeProperty::ColorBackgroundSelected,
                        )
                    })
                    .unwrap_or_else(|| Color::from_rgb8(100, 150, 255));

                // Draw selection fill
                graphics.fill(
                    Fill::NonZero,
                    Affine::translate((
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64,
                    )),
                    &Brush::Solid(selection_color.with_alpha(0.2)),
                    None,
                    &rect.to_path(0.1),
                );

                // Draw selection border
                graphics.stroke(
                    &Stroke::new(1.0),
                    Affine::translate((
                        layout.layout.location.x as f64,
                        layout.layout.location.y as f64,
                    )),
                    &Brush::Solid(selection_color.with_alpha(0.8)),
                    None,
                    &rect.to_path(0.1),
                );
            }
        }
    }
}

