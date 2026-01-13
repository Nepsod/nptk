use nalgebra::Vector2;
use nptk_core::app::context::AppContext;
use nptk_core::app::info::AppInfo;
use nptk_core::app::update::Update;
use nptk_core::layout::{Dimension, LayoutNode, LayoutStyle, StyleNode};
use nptk_core::signal::MaybeSignal;
use nptk_core::vg::kurbo::Rect;
use nptk_core::vgi::Graphics;
use nptk_core::widget::{Widget, WidgetLayoutExt};
use nptk_theme::id::WidgetId;
use nptk_theme::theme::Theme;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use vello_svg::usvg;

pub use usvg::ImageRendering;
pub use usvg::ShapeRendering;
pub use usvg::TextRendering;

mod constants;
mod loader;
mod renderer;
pub mod svg;

/// Contains the [SvgIcon] struct for representing a rendered SVG Icon.
pub use svg::SvgIcon;

/// Error type for parsing SVGs with [usvg].
pub type SvgError = usvg::Error;

/// A simple icon widget to display icons from XDG icon theme or SVG sources.
///
/// Supports both SVG icons (from XDG theme or direct SVG source) and Image icons (PNG/XPM from XDG theme).
/// Icons are loaded asynchronously and cached for performance.
///
/// ### Theming
/// The widget itself only draws the underlying icon, so theming is useless.
pub struct Icon {
    layout_style: MaybeSignal<LayoutStyle>,
    // For XDG icons
    icon_name: Option<MaybeSignal<String>>,
    size: MaybeSignal<u32>,
    icon_registry: Option<Arc<npio::service::icon::IconRegistry>>,
    icon_cache: Arc<Mutex<HashMap<(String, u32), Option<npio::service::icon::CachedIcon>>>>,
    svg_scene_cache: HashMap<String, (nptk_core::vg::Scene, f64, f64)>,
    // For SVG icons
    svg_icon: Option<MaybeSignal<SvgIcon>>,
}

impl Icon {
    /// Creates a new icon widget from an XDG icon name (e.g., "user-home", "folder", "application-pdf").
    ///
    /// The icon will be loaded from the system's XDG icon theme.
    ///
    /// # Arguments
    /// * `icon_name` - The XDG icon name (e.g., "user-home", "folder-open")
    /// * `size` - The desired icon size in pixels
    /// * `registry` - Optional shared icon registry. If None, a new registry will be created.
    pub fn new(
        icon_name: impl Into<String>,
        size: impl Into<MaybeSignal<u32>>,
        registry: Option<Arc<npio::service::icon::IconRegistry>>,
    ) -> Self {
        use npio::service::icon::IconRegistry;

        let icon_name = MaybeSignal::value(icon_name.into());
        let size = size.into();
        let registry = registry.unwrap_or_else(|| {
            Arc::new(
                IconRegistry::new().unwrap_or_else(|_| IconRegistry::default()),
            )
        });

        let initial_size = *size.get();
        let initial_icon_name = icon_name.get().clone();

        // Initialize cache
        let icon_cache = Arc::new(Mutex::new(HashMap::new()));

        // Try to load icon synchronously if possible
        let cached_icon = registry.get_icon(&initial_icon_name, initial_size);
        {
            let mut cache = icon_cache.lock().unwrap();
            cache.insert((initial_icon_name, initial_size), cached_icon);
        }

        Self {
            layout_style: LayoutStyle {
                size: Vector2::new(
                    Dimension::length(initial_size as f32),
                    Dimension::length(initial_size as f32),
                ),
                ..Default::default()
            }
            .into(),
            icon_name: Some(icon_name),
            size,
            icon_registry: Some(registry),
            icon_cache,
            svg_scene_cache: HashMap::new(),
            svg_icon: None,
        }
    }

    /// Creates a new icon widget from an SVG source string or file path.
    ///
    /// This constructor is for SVG icons that don't use XDG icon themes.
    pub fn from_svg(icon: impl Into<MaybeSignal<SvgIcon>>) -> Self {
        let icon_signal = icon.into();
        
        Self {
            layout_style: LayoutStyle {
                size: Vector2::new(Dimension::length(8.0), Dimension::length(8.0)),
                ..Default::default()
            }
            .into(),
            icon_name: None,
            size: MaybeSignal::value(0),
            icon_registry: None,
            icon_cache: Arc::new(Mutex::new(HashMap::new())),
            svg_scene_cache: HashMap::new(),
            svg_icon: Some(icon_signal),
        }
    }

    fn apply_with(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// Set the layout style for this icon.
    pub fn with_layout_style(self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) -> Self {
        self.apply_with(|s| s.layout_style = layout_style.into())
    }

    /// Extract icon rectangle from layout.
    fn icon_rect_from_layout(layout: &LayoutNode) -> Rect {
        let x = layout.layout.location.x as f64;
        let y = layout.layout.location.y as f64;
        let size = layout.layout.size.width.min(layout.layout.size.height) as f64;
        Rect::new(x, y, x + size, y + size)
    }
}

impl Widget for Icon {
    fn render(
        &mut self,
        graphics: &mut dyn Graphics,
        theme: &mut dyn Theme,
        layout_node: &LayoutNode,
        _: &mut AppInfo,
        context: AppContext,
    ) {
        let icon_rect = Self::icon_rect_from_layout(layout_node);

        // Handle SVG icons (from_svg constructor)
        if let Some(ref svg_signal) = self.svg_icon {
            let svg_icon = svg_signal.get().clone();
            
            // Scale SVG to fit layout size while maintaining aspect ratio
            let svg_width = svg_icon.width();
            let svg_height = svg_icon.height();
            let layout_width = layout_node.layout.size.width as f64;
            let layout_height = layout_node.layout.size.height as f64;
            
            if svg_width > 0.0 && svg_height > 0.0 && layout_width > 0.0 && layout_height > 0.0 {
                let target_size = layout_width.min(layout_height);
                let svg_max_dim = svg_width.max(svg_height);
                let scale = target_size / svg_max_dim;

                let scaled_width = svg_width * scale;
                let scaled_height = svg_height * scale;

                let offset_x = (layout_width - scaled_width) / 2.0;
                let offset_y = (layout_height - scaled_height) / 2.0;

                let affine = nptk_core::vg::kurbo::Affine::scale(scale)
                    .then_translate(nptk_core::vg::kurbo::Vec2::new(
                        layout_node.layout.location.x as f64 + offset_x,
                        layout_node.layout.location.y as f64 + offset_y,
                    ));

                graphics.append(&svg_icon.scene(), Some(affine));
            }
            return;
        }

        // Handle XDG theme icons
        if let (Some(ref icon_name_signal), Some(ref registry)) = (&self.icon_name, &self.icon_registry) {
            let icon_name = icon_name_signal.get().clone();
            let size = *self.size.get();

            let cache_key = (icon_name.clone(), size);
            let cached_icon = {
                let cache = self.icon_cache.lock().unwrap();
                cache.get(&cache_key).cloned().flatten()
            };

            // Request icon loading if not cached
            if cached_icon.is_none() {
                use crate::icon::loader::request_icon_loading;
                request_icon_loading(
                    self.icon_cache.clone(),
                    registry.clone(),
                    icon_name.clone(),
                    size,
                    context,
                );
            }

            // Render icon or fallback
            if let Some(icon) = cached_icon {
                use crate::icon::renderer::render_cached_icon;
                render_cached_icon(
                    graphics,
                    theme,
                    self.widget_id(),
                    icon,
                    icon_rect,
                    &mut self.svg_scene_cache,
                );
            } else {
                use crate::icon::renderer::render_fallback_icon;
                render_fallback_icon(graphics, theme, self.widget_id(), icon_rect);
            }
        }
    }

    fn layout_style(&self) -> StyleNode {
        StyleNode {
            style: self.layout_style.get().clone(),
            children: Vec::new(),
        }
    }

    fn update(&mut self, layout: &LayoutNode, context: AppContext, _: &mut AppInfo) -> Update {
        // Handle reactive icon_name and size changes for XDG icons
        if let (Some(ref icon_name_signal), Some(ref registry)) = (&self.icon_name, &self.icon_registry) {
            let icon_name = icon_name_signal.get().clone();
            let size = *self.size.get();

            let cache_key = (icon_name.clone(), size);
            let needs_loading = {
                let cache = self.icon_cache.lock().unwrap();
                !cache.contains_key(&cache_key)
            };

            if needs_loading {
                use crate::icon::loader::request_icon_loading;
                request_icon_loading(
                    self.icon_cache.clone(),
                    registry.clone(),
                    icon_name,
                    size,
                    context,
                );
            }

            // Update layout size if size changed
            let current_size = *self.size.get();
            let expected_size = current_size as f32;
            let layout_style_clone = self.layout_style.get().clone();
            // Simple comparison: update if size changed (check if we need to update)
            // For now, always update layout when size changes
            self.layout_style = LayoutStyle {
                size: Vector2::new(
                    Dimension::length(expected_size),
                    Dimension::length(expected_size),
                ),
                ..layout_style_clone
            }
            .into();
        }

        Update::empty()
    }

    fn widget_id(&self) -> WidgetId {
        WidgetId::new("nptk-widgets", "Icon")
    }
}

impl WidgetLayoutExt for Icon {
    fn set_layout_style(&mut self, layout_style: impl Into<MaybeSignal<LayoutStyle>>) {
        self.layout_style = layout_style.into();
    }
}
