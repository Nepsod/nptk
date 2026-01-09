//! File icon widget for rendering file icons with async loading and caching.
//!
//! This module provides a widget that loads icons asynchronously, caches them
//! per (path, size) key, and caches parsed SVG scenes to avoid re-parsing.

mod constants;
mod loader;
pub mod renderer;
mod theme;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::Rect;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use npio::service::icon::{CachedIcon, IconRegistry};
use nptk_services::filesystem::entry::FileEntry;
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;

use crate::file_icon::loader::request_icon_loading;
use crate::file_icon::renderer::{render_cached_icon, render_fallback_icon};

/// Widget for rendering file icons with async loading and caching.
///
/// This widget loads icons asynchronously, caches them per (path, size) key,
/// and caches parsed SVG scenes to avoid re-parsing.
pub struct FileIcon {
    /// File entry.
    entry: MaybeSignal<FileEntry>,
    /// Icon size.
    size: MaybeSignal<u32>,
    /// Layout style.
    layout_style: MaybeSignal<LayoutStyle>,
    /// Icon registry for loading icons.
    icon_registry: Arc<IconRegistry>,
    /// Icon cache: (path, size) -> CachedIcon.
    icon_cache: Arc<Mutex<HashMap<(PathBuf, u32), Option<CachedIcon>>>>,
    /// SVG scene cache: SVG source string -> (Scene, width, height).
    svg_scene_cache: HashMap<String, (nptk_core::vg::Scene, f64, f64)>,
}

impl FileIcon {
    /// Create a new file icon widget.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to load icon for
    /// * `size` - The icon size in pixels
    pub fn new(entry: FileEntry, size: u32) -> Self {
        let icon_registry = Arc::new(
            IconRegistry::new().unwrap_or_else(|_| IconRegistry::default())
        );
        let icon_cache = Arc::new(Mutex::new(HashMap::new()));

        Self {
            entry: entry.into(),
            size: size.into(),
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::length(size as f32), Dimension::length(size as f32)),
                ..Default::default()
            }
            .into(),
            icon_registry,
            icon_cache,
            svg_scene_cache: HashMap::new(),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Set the file entry.
    pub fn with_entry(self, entry: impl Into<MaybeSignal<FileEntry>>) -> Self {
        self.apply_with(|s| s.entry = entry.into())
    }

    /// Set the icon size.
    pub fn with_size(self, size: u32) -> Self {
        self.apply_with(|s| s.set_size(size))
    }

    fn set_size(&mut self, size: u32) {
        self.size = size.into();
        self.layout_style = LayoutStyle {
            size: Vector2::new(Dimension::length(size as f32), Dimension::length(size as f32)),
            ..Default::default()
        }
        .into();
    }

    /// Extract icon rectangle from layout.
    fn icon_rect_from_layout(layout: &LayoutNode) -> Rect {
        let x = layout.layout.location.x as f64;
        let y = layout.layout.location.y as f64;
        let size = layout.layout.size.width.min(layout.layout.size.height) as f64;
        Rect::new(x, y, x + size, y + size)
    }
}

impl Widget for FileIcon {
    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "FileIcon")
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: vec![],
        }
    }

    fn update(&mut self, _: &LayoutNode, _: AppContext, _: &mut AppInfo) -> Update {
        Update::empty()
    }

    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout: &LayoutNode,
        _: &mut AppInfo,
        context: AppContext,
    ) {
        let entry = self.entry.get().clone();
        let path = entry.path.clone();
        let size = *self.size.get();
        let icon_rect = Self::icon_rect_from_layout(layout);

        let cache_key = (path.clone(), size);
        let cached_icon = {
            let cache = self.icon_cache.lock().unwrap();
            cache.get(&cache_key).and_then(|opt| opt.clone())
        };

        // Request icon loading if not cached
        if cached_icon.is_none() {
            request_icon_loading(
                self.icon_cache.clone(),
                self.icon_registry.clone(),
                entry.clone(),
                size,
                context,
            );
        }

        // Render icon or fallback
        if let Some(icon) = cached_icon {
            render_cached_icon(
                graphics,
                theme,
                self.widget_id(),
                icon,
                icon_rect,
                &entry,
                &mut self.svg_scene_cache,
            );
        } else {
            render_fallback_icon(
                graphics,
                theme,
                self.widget_id(),
                icon_rect,
                &entry,
            );
        }
    }
}

impl WidgetLayoutExt for FileIcon {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
