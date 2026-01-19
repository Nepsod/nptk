#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Maximum number of layout computations to cache
const MAX_LAYOUT_CACHE_SIZE: usize = 10;

mod events;
mod measure_bridge;
mod render;
#[cfg(all(target_os = "linux", feature = "wayland"))]
mod wayland;

use crate::app::context::AppContext;
use crate::layout::{InvalidationTracker, LayoutContext, LayoutDirection, LayoutNode};
use crate::vgi::graphics_from_scene;
use crate::vgi::{DeviceHandle, GpuContext};
use crate::vgi::{Renderer, RendererOptions, Scene, Surface, SurfaceTrait};
use crate::vgi::scene::DirtyRegionTracker;
use nalgebra::Vector2;
use nptk_services::settings::SettingsRegistry;
use taffy::prelude::*;
use taffy::{
    AvailableSpace, Dimension, NodeId, PrintTree, Size, Style, TaffyResult, TaffyTree,
    TraversePartialTree,
};
use vello::wgpu::{CommandEncoderDescriptor, TextureFormat, TextureViewDescriptor};
use vello::{AaConfig, AaSupport, RenderParams};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::font_ctx::FontContext;
use crate::app::info::AppInfo;
#[cfg(target_os = "linux")]
use crate::app::info::WindowIdentity;
use crate::app::update::{Update, UpdateManager};
use crate::config::MayConfig;
use crate::layout::StyleNode;
use crate::platform::Platform;
use crate::plugin::PluginManager;
use crate::widget::Widget;
#[cfg(target_os = "linux")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use taffy::style::Display;

/// The core application handler. You should use [MayApp](crate::app::MayApp) instead for running applications.
pub struct AppHandler<W, S, F>
where
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    config: MayConfig,
    attrs: WindowAttributes,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    scene: Scene,
    surface: Option<Surface>,
    taffy: TaffyTree,
    window_node: NodeId,
    builder: F,
    state: Option<S>,
    widget: Option<W>,
    info: AppInfo,
    gpu_context: Option<Arc<GpuContext>>,
    update: UpdateManager,
    last_update: Instant,
    plugins: PluginManager,
    selected_device: usize,
    /// Tracks whether async initialization is complete
    async_init_complete: Arc<AtomicBool>,
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    wayland_pressed_keys: HashSet<u32>,
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    xkb_keymap: crate::app::keymap::XkbKeymapManager,
    text_render: crate::text_render::TextRenderContext,
    menu_manager: crate::menu::ContextMenuState,
    shortcut_registry: crate::shortcut::ShortcutRegistry,
    action_callbacks: crate::app::action::ActionCallbackManager,
    popup_manager: crate::app::popup::PopupManager,
    tooltip_request_manager: crate::app::tooltip::TooltipRequestManager,
    tooltip_manager: crate::app::tooltip::TooltipManager,
    status_bar: crate::app::status_bar::StatusBarManager,
    popup_windows: std::collections::HashMap<WindowId, PopupWindow>,
    /// Native Wayland popups (indexed by surface key u32)
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    wayland_popups: std::collections::HashMap<u32, PopupWindow>,
    /// Counter for generating unique Wayland popup IDs
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    wayland_popup_id_counter: u32,
    settings: Arc<SettingsRegistry>,
    /// Cached theme reference for rendering (updated when theme changes)
    theme_cache: Option<Arc<std::sync::RwLock<Box<dyn nptk_theme::theme::Theme + Send + Sync>>>>,
    /// Receiver for theme change notifications
    theme_change_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// Tracks dirty regions to avoid unnecessary scene resets
    dirty_region_tracker: DirtyRegionTracker,
    /// Cache for layout computation to avoid redundant calculations
    layout_cache: Option<(u64, LayoutNode)>, // (hash, cached_layout)
    /// Counter for cache management
    layout_cache_hits: usize,
    /// Channel for receiving the GPU context after async initialization
    gpu_init_rx: Option<std::sync::mpsc::Receiver<Result<(Arc<GpuContext>, DeviceHandle), Box<dyn std::error::Error + Send + Sync>>>>,
    /// Tracks which widgets need layout updates (invalidation)
    invalidation_tracker: InvalidationTracker,
    /// Last known window size to detect actual size changes
    last_window_size: (u32, u32),
    /// Timestamp of last resize event for throttling
    last_resize_time: Instant,
    /// Pending resize size (accumulated during throttling period)
    pending_resize: Option<(u32, u32)>,
    /// Cached root child node ID to avoid repeated lookups
    cached_root_child: Option<NodeId>,
}

struct PopupWindow {
    /// Winit window (only for X11/Winit-based popups, None for native Wayland)
    window: Option<Arc<Window>>,
    renderer: Renderer,
    scene: Scene,
    surface: Surface,
    taffy: TaffyTree,
    root_node: NodeId,
    widget: Box<dyn Widget>,
    info: AppInfo,
    config: MayConfig, // Each window needs its own config copy/ref for theme access
    /// Scale factor for HiDPI (1.0 for X11/Winit, 2.0 or higher for Wayland HiDPI)
    scale_factor: f32,
}
impl<W, S, F> AppHandler<W, S, F>
where
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    /// Create a new handler with given window attributes, config, widget and state.
    pub fn new(
        attrs: WindowAttributes,
        config: MayConfig,
        builder: F,
        state: S,
        font_context: FontContext,
        update: UpdateManager,
        plugins: PluginManager,
        settings: Arc<SettingsRegistry>,
    ) -> Self {
        let mut taffy = TaffyTree::with_capacity(16);

        // gets configured on resume
        let window_node = taffy
            .new_leaf(Style::default())
            .expect("Failed to create window node");

        let size = config.window.size;
        let backend = config.render.backend.clone();

        #[cfg(all(target_os = "linux", feature = "wayland"))]
        let xkb_keymap = crate::app::keymap::XkbKeymapManager::new().unwrap_or_else(|e| {
            log::warn!("Failed to create XKB keymap manager: {}", e);
            crate::app::keymap::XkbKeymapManager::default()
        });

        // Subscribe to theme changes
        let theme_change_rx = {
            let manager_read = config.theme_manager.read().unwrap();
            Some(manager_read.subscribe_theme_changes())
        };
        
        // Cache the current theme reference
        let theme_cache = {
            let manager_read = config.theme_manager.read().unwrap();
            Some(manager_read.current_theme())
        };

        Self {
            attrs,
            window: None,
            renderer: None,
            config,
            scene: Scene::new(backend, 0, 0), // Will be updated on resize
            surface: None,
            taffy,
            theme_cache,
            theme_change_rx,
            widget: None,
            info: AppInfo {
                font_context,
                size,
                ..Default::default()
            },
            window_node,
            builder,
            state: Some(state),
            gpu_context: None,
            update,
            last_update: Instant::now(),
            plugins,
            selected_device: 0,
            async_init_complete: Arc::new(AtomicBool::new(false)),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_pressed_keys: HashSet::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            xkb_keymap,
            text_render: crate::text_render::TextRenderContext::new(),
            menu_manager: crate::menu::ContextMenuState::new(),
            shortcut_registry: crate::shortcut::ShortcutRegistry::new(),
            action_callbacks: crate::app::action::ActionCallbackManager::new(),
            popup_manager: crate::app::popup::PopupManager::new(),
            tooltip_request_manager: crate::app::tooltip::TooltipRequestManager::new(),
            tooltip_manager: crate::app::tooltip::TooltipManager::new(),
            status_bar: crate::app::status_bar::StatusBarManager::default(),
            popup_windows: std::collections::HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_popups: std::collections::HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_popup_id_counter: 0,
            settings,
            dirty_region_tracker: DirtyRegionTracker::new(),
            layout_cache: None,
            layout_cache_hits: 0,
            gpu_init_rx: None,
            invalidation_tracker: InvalidationTracker::new(),
            last_window_size: (size.x as u32, size.y as u32),
            last_resize_time: Instant::now(),
            pending_resize: None,
            cached_root_child: None,
        }
    }

    /// Get the application context.
    /// Returns None if GPU context is not available (e.g., during shutdown).
    pub fn context(&self) -> Option<AppContext> {
        self.gpu_context.as_ref().map(|gpu_ctx| {
            AppContext::new(
                self.update.clone(),
                self.info.diagnostics.clone(),
                gpu_ctx.clone(),
                self.info.focus_manager.clone(),
                self.menu_manager.clone(),
                self.shortcut_registry.clone(),
                self.action_callbacks.clone(),
                self.popup_manager.clone(),
                self.tooltip_request_manager.clone(),
                self.status_bar.clone(),
                self.settings.clone(),
            )
        })
    }

    /// Add the parent node and its children to the layout tree.
    fn layout_widget(&mut self, parent: NodeId, style: &StyleNode) -> TaffyResult<()> {
        // If this widget itself has Display::None, don't add it to the layout tree at all
        // This ensures hidden widgets don't take up any space
        if style.style.display == Display::None {
            return Ok(());
        }

        // Note: Taffy 0.8 doesn't have new_leaf_with_measure API.
        // Measure functions are used during StyleNode building to set better initial sizes.
        // For now, we create regular leaf nodes. Future Taffy versions may support
        // measure functions directly in the layout pass.
        let node = self.taffy.new_leaf(style.style.clone().into())?;
        self.taffy.add_child(parent, node)?;

        // Only add children that are not Display::None to the Taffy tree
        // This ensures hidden widgets don't take up space
        for child in &style.children {
            if child.style.display != Display::None {
                self.layout_widget(node, child)?;
            }
        }

        Ok(())
    }

    /// Compute a hash for the current layout state to enable caching
    fn compute_layout_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // Hash window size
        let (width, height) = if let Some(surface) = &self.surface {
            surface.size()
        } else if let Some(window) = self.window.as_ref() {
            let s = window.inner_size();
            (s.width, s.height)
        } else {
            let s = self.config.window.size;
            (s.x as u32, s.y as u32)
        };
        
        width.hash(&mut hasher);
        height.hash(&mut hasher);
        
        // Hash widget structure and style information
        if let Some(widget) = &self.widget {
            // Hash widget ID
            widget.widget_id().hash(&mut hasher);
            
            // Hash layout style to detect style changes
            let context = LayoutContext::unbounded();
            let style = widget.layout_style(&context);
            self.hash_style_node(&style, &mut hasher);
        }
        
        hasher.finish()
    }

    /// Recursively hash a style node and its children for layout cache invalidation
    fn hash_style_node(&self, style: &crate::layout::StyleNode, hasher: &mut std::collections::hash_map::DefaultHasher) {
        use std::hash::Hash;
        
        // Hash display property (critical for layout)
        std::mem::discriminant(&style.style.display).hash(hasher);
        
        // Hash position using discriminant
        std::mem::discriminant(&style.style.position).hash(hasher);
        
        // Hash flex properties using discriminant for enums
        std::mem::discriminant(&style.style.flex_direction).hash(hasher);
        std::mem::discriminant(&style.style.flex_wrap).hash(hasher);
        
        // Hash flex grow/shrink as bits
        style.style.flex_grow.to_bits().hash(hasher);
        style.style.flex_shrink.to_bits().hash(hasher);
        
        // Hash children count and recurse
        style.children.len().hash(hasher);
        for child in &style.children {
            self.hash_style_node(child, hasher);
        }
    }

    /// Compute the layout of the root node and its children with caching.
    fn compute_layout_cached(&mut self) -> TaffyResult<LayoutNode> {
        let layout_hash = self.compute_layout_hash();
        
        // Check if we have a cached layout for this hash
        if let Some((cached_hash, ref cached_layout)) = &self.layout_cache {
            if *cached_hash == layout_hash {
                log::trace!("Using cached layout (hash: {})", layout_hash);
                self.layout_cache_hits += 1;
                return Ok(cached_layout.clone());
            }
        }
        
        log::trace!("Computing new layout (hash: {})", layout_hash);
        
        // Compute fresh layout
        self.compute_layout()?;
        
        if let Some(widget) = &self.widget {
            let context = LayoutContext::unbounded();
            let style = widget.layout_style(&context);
            let layout_node = self.collect_layout(self.window_node, &style)?;
            
            // Cache the result
            self.layout_cache = Some((layout_hash, layout_node.clone()));
            
            Ok(layout_node)
        } else {
            // No widget, return empty layout
            Ok(LayoutNode {
                layout: Default::default(),
                children: vec![],
            })
        }
    }
    fn compute_layout(&mut self) -> TaffyResult<()> {
        // Determine current size from Wayland surface if present, otherwise from window,
        // otherwise fall back to configured size.
        let (width, height) = if let Some(surface) = &self.surface {
            surface.size()
        } else if let Some(window) = self.window.as_ref() {
            let s = window.inner_size();
            (s.width, s.height)
        } else {
            let s = self.config.window.size;
            (s.x as u32, s.y as u32)
        };

        log::debug!(
            "compute_layout: computing layout for window node with AvailableSpace {}x{}",
            width,
            height
        );

        self.taffy.compute_layout(
            self.window_node,
            Size::<AvailableSpace> {
                width: AvailableSpace::Definite(width as f32),
                height: AvailableSpace::Definite(height as f32),
            },
        )?;
        
        Ok(())
    }

    /// Collect the computed layout of the given node and its children. Make sure to call [AppHandler::compute_layout] before, to not get dirty results.
    ///
    /// This method converts Taffy's relative positions (relative to parent's content area) to absolute positions
    /// by accumulating parent positions and paddings as we traverse the tree.
    fn collect_layout(&mut self, node: NodeId, style: &StyleNode) -> TaffyResult<LayoutNode> {
        // Get layout direction from context (default to LTR)
        let context = LayoutContext::unbounded();
        let direction = context.direction;
        self.collect_layout_impl(node, style, 0.0, 0.0, direction)
    }

    /// Internal implementation that accumulates parent positions.
    /// parent_x and parent_y represent the absolute position of the parent's content area (after padding).
    fn collect_layout_impl(
        &mut self,
        node: NodeId,
        style: &StyleNode,
        parent_x: f32,
        parent_y: f32,
        direction: crate::layout::LayoutDirection,
    ) -> TaffyResult<LayoutNode> {
        // Pre-allocate children Vec with capacity hint to reduce allocations
        let visible_count = style.children.iter()
            .filter(|cs| cs.style.display != Display::None)
            .count();
        let mut children = Vec::with_capacity(visible_count);
        let taffy_child_count = self.taffy.child_count(node);
        
        // Early return if no children to process
        if taffy_child_count == 0 && visible_count == 0 {
            let relative_layout = *self.taffy.get_final_layout(node);
            let absolute_x = parent_x + relative_layout.location.x;
            let absolute_y = parent_y + relative_layout.location.y;
            let mut absolute_layout = relative_layout;
            absolute_layout.location.x = absolute_x;
            absolute_layout.location.y = absolute_y;
            return Ok(LayoutNode {
                layout: absolute_layout,
                children,
            });
        }

        // Get the relative layout from Taffy
        let relative_layout = *self.taffy.get_final_layout(node);

        // Convert relative position to absolute by adding parent's content area position
        // Taffy positions are relative to parent's content area (after padding)
        // So: absolute = parent_content + relative
        let absolute_x = parent_x + relative_layout.location.x;
        let absolute_y = parent_y + relative_layout.location.y;

        // Create absolute layout
        let mut absolute_layout = relative_layout;
        absolute_layout.location.x = absolute_x;
        absolute_layout.location.y = absolute_y;

        // Calculate this node's content area position (absolute position)
        // This will be the parent position for children
        // Extract padding values from LengthPercentage
        // We extract from style.style.padding which is still our LayoutStyle type
        // The content area starts after the padding (absolute position + padding = content area start)
        let child_content_x = absolute_x;
        let child_content_y = absolute_y;

        // The style includes ALL children, but Taffy only has visible ones (Display::None filtered out during build)
        // We need to iterate through style children and skip Display::None ones, collecting from Taffy in order
        let mut taffy_index = 0;
        for child_style in &style.children {
            // Skip Display::None children - they weren't added to Taffy
            if child_style.style.display == Display::None {
                continue;
            }

            // Early exit if we've collected all Taffy children
            if taffy_index >= taffy_child_count {
                break;
            }

            // Collect this visible child from Taffy, passing our content area position as the new parent content position
            let child_node = self.taffy.child_at_index(node, taffy_index)?;
            children.push(self.collect_layout_impl(
                child_node,
                child_style,
                child_content_x,
                child_content_y,
                direction,
            )?);
            taffy_index += 1;
        }

        // Only warn if significant mismatch (more than 1 child difference) to reduce log spam
        // This is rare and usually indicates a bug, so we keep the check but make it debug-level
        if taffy_index < taffy_child_count && (taffy_child_count - taffy_index) > 1 {
            log::debug!(
                "Layout mismatch: collected {} children from Taffy, but Taffy has {} children",
                taffy_index,
                taffy_child_count
            );
        }

        // Skip logging in release builds to reduce overhead
        #[cfg(debug_assertions)]
        if !children.is_empty() {
            log::debug!(
                "Collected layout for node {:?}: pos=({:.1}, {:.1}), size=({:.1}, {:.1}), {} children",
                node,
                absolute_layout.location.x,
                absolute_layout.location.y,
                absolute_layout.size.width,
                absolute_layout.size.height,
                children.len()
            );
        }

        Ok(LayoutNode {
            layout: absolute_layout,
            children,
        })
    }

    /// Request a window redraw.
    fn request_redraw(&self) {
        log::trace!("Requesting redraw via update flag");

        // Set the DRAW flag which will be processed by user_event or about_to_wait
        self.update.insert(Update::DRAW);
        
        // For winit windows, also call request_redraw in case RedrawRequested works
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    /// Update the app and process events.
    /// This is called by the winit event loop periodically.
    pub fn update(&mut self, event_loop: &ActiveEventLoop) {
        log::trace!("update() called");

        // For Wayland, process events first to trigger frame callbacks
        // CRITICAL: Detect resize BEFORE layout computation so layout uses new size
        let platform = Platform::detect();
        // Process pending resize if enough time has passed (throttling)
        const RESIZE_THROTTLE_MS: u64 = 16; // ~60fps
        if let Some(pending_size) = self.pending_resize {
            let time_since_last_resize = self.last_update.duration_since(self.last_resize_time).as_millis() as u64;
            if time_since_last_resize >= RESIZE_THROTTLE_MS {
                self.update_window_node_size(pending_size.0, pending_size.1);
                self.info.size = nalgebra::Vector2::new(pending_size.0 as f64, pending_size.1 as f64);
                self.last_window_size = pending_size;
                self.last_resize_time = self.last_update;
                self.pending_resize = None;
                self.update.insert(Update::DRAW | Update::LAYOUT);
            }
        }

        if platform == Platform::Wayland {
            if let Some(ref mut surface) = self.surface {
                if surface.needs_event_dispatch() {
                    let size_before = surface.size();
                    match surface.dispatch_events() {
                        Ok(needs_redraw) => {
                            let size_after = surface.size();
                            if size_before != size_after {
                                // Throttle Wayland resize events too
                                let now = Instant::now();
                                let time_since_last_resize = now.duration_since(self.last_resize_time).as_millis() as u64;
                                let size_changed_significantly = size_after != self.last_window_size &&
                                    ((size_after.0 as i32 - self.last_window_size.0 as i32).abs() > 1 ||
                                     (size_after.1 as i32 - self.last_window_size.1 as i32).abs() > 1);

                                if size_changed_significantly || time_since_last_resize >= RESIZE_THROTTLE_MS {
                                    self.update_window_node_size(size_after.0, size_after.1);
                                    self.info.size = nalgebra::Vector2::new(size_after.0 as f64, size_after.1 as f64);
                                    self.last_window_size = (size_after.0, size_after.1);
                                    self.last_resize_time = now;
                                    self.pending_resize = None;
                                    self.update.insert(Update::DRAW | Update::LAYOUT);
                                } else {
                                    self.pending_resize = Some((size_after.0, size_after.1));
                                    self.update.insert(Update::DRAW);
                                }
                            } else if needs_redraw {
                                log::trace!("Wayland events triggered redraw");
                                self.update.insert(Update::DRAW);
                            }
                        },
                        Err(err) => {
                            log::info!("Wayland surface dispatch reported close: {}", err);
                            self.update.insert(Update::EXIT);
                        },
                    }
                }
            }
            
            if let Some(ref mut surface) = self.surface {
                // Fallback: if the Wayland surface has been configured, force a first draw so we attach a buffer.
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                {
                    if let crate::vgi::Surface::Wayland(ref wayland_surface) = surface {
                        // Keep scheduling redraws until the first frame callback is observed.
                        if wayland_surface.is_configured() && !wayland_surface.first_frame_seen() {
                            log::trace!(
                                "Wayland: first frame not seen yet; scheduling redraw fallback"
                            );
                            self.update.insert(Update::FORCE | Update::DRAW);
                        }
                    }
                }
            }
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            {
                self.process_wayland_input_events();
            }
        }

        // Update window identity periodically to ensure it's set (important for Wayland)
        // This ensures the Wayland surface protocol ID is available for menu registration
        #[cfg(target_os = "linux")]
        {
            if self.surface.is_some() || self.window.is_some() {
                self.update_window_identity();
            }
        }

        self.update_internal(event_loop);
    }

    /// Update the app and process events (internal implementation).
    fn update_internal(&mut self, event_loop: &ActiveEventLoop) {
        // Process popup requests first (in case they were requested during widget updates)
        // This ensures popups are created in the same update cycle
        self.process_popup_requests(event_loop);
        
        // Check for theme changes and trigger redraw if needed
        self.check_theme_changes();

        // Check for async GPU initialization completion
        if let Some(ref rx) = self.gpu_init_rx {
            if let Ok(result) = rx.try_recv() {
                // Clear receiver
                self.gpu_init_rx = None;

                match result {
                    Ok((gpu_ctx, device_handle)) => {
                        log::debug!("Async GPU initialization finished successfully");
                        log::debug!("Creating renderer...");
                        self.create_renderer(&device_handle);
                        self.gpu_context = Some(gpu_ctx);
                        self.async_init_complete.store(true, Ordering::Relaxed);

                        let platform = Platform::detect();
                         if platform != Platform::Wayland {
                            self.update.insert(Update::FORCE);
                            if let Some(window) = &self.window {
                                log::debug!("Requesting initial redraw for winit window");
                                window.request_redraw();
                            }
                        } else {
                            log::debug!("Wayland: Update flags should be handled by configure");
                            // Force a draw anyway to be safe
                            self.update.insert(Update::FORCE | Update::DRAW);
                        }
                    },
                    Err(e) => {
                        log::error!("Async GPU initialization failed: {}", e);
                    }
                }
            }
        }
        
        self.update_plugins(event_loop);

        let mut layout_node = self.ensure_layout_initialized();
        layout_node = self.update_layout_if_needed(layout_node);

        // Check if cursor is over menu for masking during render (but not during update/events)
        let original_cursor_pos = self.info.cursor_pos;
        let cursor_over_menu = self.is_cursor_over_menu();
        
        // Update widget - cursor is NOT masked here so event handling works correctly
        // Cursor will be masked during render phase only
        self.update_widget(&layout_node);

        // Process tooltip requests AFTER widget updates (so requests from widgets are processed)
        self.process_tooltip_requests();

        // If widget update set LAYOUT or FORCE flags, rebuild the layout immediately
        // This ensures visibility changes take effect in the same frame
        layout_node = self.update_layout_if_needed(layout_node);

        let update_flags = self.update.get();
        log::trace!(
            "Update flags: {:?}, FORCE: {}, DRAW: {}",
            update_flags,
            update_flags.intersects(Update::FORCE),
            update_flags.intersects(Update::DRAW)
        );

        if update_flags.intersects(Update::FORCE | Update::DRAW) {
            log::trace!(
                "Rendering frame (FORCE={}, DRAW={})",
                update_flags.intersects(Update::FORCE),
                update_flags.intersects(Update::DRAW)
            );
            // Always use the latest layout_node that was just updated
            // The layout_node parameter is already the latest one from update_layout_if_needed
            // Pass cursor masking info so render can handle menu cursor restoration
            self.render_frame(&layout_node, event_loop, cursor_over_menu, original_cursor_pos);
        }

        // Cursor position will be reset in info.reset() below

        self.handle_update_flags(event_loop);
        self.info.reset();
        self.update_diagnostics();
    }

    fn update_diagnostics(&mut self) {
        if self.last_update.elapsed() >= Duration::from_secs(1) {
            self.last_update = Instant::now();
            self.info.diagnostics.updates_per_sec =
                (self.info.diagnostics.updates_per_sec + self.info.diagnostics.updates) / 2;
            self.info.diagnostics.updates = 0;
        } else {
            self.info.diagnostics.updates += 1;
        }

        log::trace!("Updates per sec: {}", self.info.diagnostics.updates_per_sec);
    }

    /// Update plugins with current state.
    /// Check for theme changes and trigger redraw if the theme has changed.
    fn check_theme_changes(&mut self) {
        // Check for theme change notifications
        if let Some(ref mut rx) = self.theme_change_rx {
            // Non-blocking check for theme changes
            while let Ok(_variant) = rx.try_recv() {
                log::debug!("Theme changed, triggering redraw");
                // Update cached theme reference
                self.theme_cache = {
                    let manager_read = self.config.theme_manager.read().unwrap();
                    Some(manager_read.current_theme())
                };
                // Request redraw to apply new theme
                self.update.insert(Update::DRAW);
            }
        }

        // Update active theme transitions
        if let Ok(mut manager) = self.config.theme_manager.write() {
            let had_transition = manager.has_active_transition();
            manager.update_transition();
            
            // If transition is active or just completed, trigger redraw
            if had_transition || manager.has_active_transition() {
                self.update.insert(Update::DRAW);
            }

            // Check for hot reload file changes
            match manager.check_and_reload() {
                Ok(true) => {
                    // Reload was triggered, update cached theme reference
                    self.theme_cache = Some(manager.current_theme());
                    // Trigger redraw to apply reloaded theme
                    self.update.insert(Update::DRAW);
                },
                Ok(false) => {
                    // No reload needed
                },
                Err(e) => {
                    log::warn!("Failed to reload theme: {}", e);
                },
            }
        }
    }

    fn update_plugins(&mut self, event_loop: &ActiveEventLoop) {
        // For Wayland, window is None - skip plugin updates that require window
        let platform = Platform::detect();
        if platform == Platform::Wayland && self.window.is_none() {
            // Skip plugin updates for Wayland when no winit window exists
            return;
        }

        // For Winit, window must exist - if it doesn't, something is wrong
        let window_opt = self.window.as_ref();
        if window_opt.is_none() {
            // Window should exist for non-Wayland platforms
            // But don't panic - just skip plugin updates until window is created
            log::debug!("Window not yet created, skipping plugin updates");
            return;
        }

        // Check if renderer, surface, and gpu_context are initialized
        // If not, skip plugin updates until initialization is complete
        if self.renderer.is_none() || self.surface.is_none() || self.gpu_context.is_none() {
            log::debug!(
                "Renderer/surface/gpu_context not yet initialized, skipping plugin updates"
            );
            return;
        }

        self.plugins.run(|pl| {
            pl.on_update(
                &mut self.config,
                window_opt.expect("Window not initialized"),
                self.renderer.as_mut().expect("Renderer not initialized"),
                &mut self.scene,
                self.surface.as_mut().expect("Surface not initialized"),
                &mut self.taffy,
                self.window_node,
                &mut self.info,
                self.gpu_context
                    .as_ref()
                    .expect("GPU context not initialized"),
                &self.update,
                &mut self.last_update,
                event_loop,
            )
        });
    }

    /// Ensure layout is initialized, returning the current layout node.
    /// Optimized to use cached root child when available.
    fn ensure_layout_initialized(&mut self) -> LayoutNode {
        if self.taffy.child_count(self.window_node) == 0 {
            self.setup_initial_layout();
        }

        let child_count = self.taffy.child_count(self.window_node);
        if child_count == 0 {
            // Root widget has no children (all filtered out) - return empty layout
            return LayoutNode {
                layout: *self.taffy.get_final_layout(self.window_node),
                children: vec![],
            };
        }
        
        // Use cached root child if available
        let root_child = if let Some(cached_child) = self.cached_root_child {
            cached_child
        } else {
            let child = self.taffy.child_at_index(self.window_node, 0).unwrap();
            self.cached_root_child = Some(child);
            child
        };
        
        // Compute style (can't cache StyleNode due to function pointer)
        let context = LayoutContext::unbounded();
        let style = self.widget.as_ref().unwrap().layout_style(&context);
        
        self.collect_layout(root_child, &style)
            .expect("Failed to collect layout")
    }

    /// Set up the initial layout tree.
    fn setup_initial_layout(&mut self) {
        log::debug!("Setting up layout...");
        let context = LayoutContext::unbounded();
        let style = self.widget.as_ref().unwrap().layout_style(&context);
        self.layout_widget(self.window_node, &style)
            .expect("Failed to layout window");
        self.compute_layout().expect("Failed to compute layout");
        
        // Cache root child node ID
        if self.taffy.child_count(self.window_node) > 0 {
            if let Ok(root_child) = self.taffy.child_at_index(self.window_node, 0) {
                self.cached_root_child = Some(root_child);
            }
        }
        
        self.update.insert(Update::FORCE);
    }

    /// Update layout if needed, returning the updated layout node.
    /// Optimized to avoid full rebuilds when only window size changes.
    fn update_layout_if_needed(&mut self, layout_node: LayoutNode) -> LayoutNode {
        if !self.update.get().intersects(Update::LAYOUT | Update::FORCE) {
            return layout_node;
        }

        // Reset metrics for this layout pass
        self.invalidation_tracker.reset_metrics();
        let start_time = std::time::Instant::now();

        // Get current window size (cached check first to avoid repeated lookups)
        let (current_width, current_height): (u32, u32) = if let Some(surface) = &self.surface {
            // Prefer surface size (more accurate for Wayland)
            let (w, h) = surface.size();
            (w, h)
        } else if let Some(window) = self.window.as_ref() {
            let s = window.inner_size();
            (s.width, s.height)
        } else {
            let s = self.config.window.size;
            (s.x as u32, s.y as u32)
        };

        // Check if size actually changed (early exit if no change)
        let size_changed = (current_width, current_height) != self.last_window_size;
        
        // If FORCE flag is set, always rebuild (structure might have changed)
        // If size didn't change and FORCE is not set, we might be able to skip
        let needs_full_rebuild = self.update.get().intersects(Update::FORCE) || !size_changed;
        
        if size_changed {
            // Update window node size immediately
            self.update_window_node_size(current_width, current_height);
            self.info.size = nalgebra::Vector2::new(current_width as f64, current_height as f64);
            self.last_window_size = (current_width, current_height);
            
            // If only size changed and tree structure is intact, just recompute layout
            // This is much faster than rebuilding the entire tree
            if !needs_full_rebuild && self.taffy.child_count(self.window_node) > 0 {
                // Fast path: only recompute layout without rebuilding tree
                if let Err(e) = self.compute_layout() {
                    log::warn!("Fast layout recompute failed, falling back to rebuild: {}", e);
                    // Fall through to full rebuild
                } else {
                    // Successfully recomputed, collect new layout using cached root child
                    // Use cached root child if available, otherwise look it up and cache it
                    let root_child = if let Some(cached_child) = self.cached_root_child {
                        cached_child
                    } else {
                        match self.taffy.child_at_index(self.window_node, 0) {
                            Ok(child) => {
                                self.cached_root_child = Some(child);
                                child
                            },
                            Err(_) => {
                                // Fall through to full rebuild
                                return layout_node;
                            }
                        }
                    };
                    
                    // Compute style only when needed (can't cache StyleNode due to function pointer)
                    let context = LayoutContext::unbounded();
                    let style = self.widget.as_ref().unwrap().layout_style(&context);
                    
                    if let Ok(new_layout) = self.collect_layout(root_child, &style) {
                        // Clear the LAYOUT flag
                        if self.update.get().intersects(Update::LAYOUT) {
                            self.update.remove(Update::LAYOUT);
                        }
                        
                        let layout_time = start_time.elapsed();
                        self.invalidation_tracker
                            .metrics_mut()
                            .record_layout_time(layout_time.as_secs_f64() * 1000.0);
                        self.invalidation_tracker.metrics_mut().record_recomputation();
                        
                        return new_layout;
                    }
                }
            }
        }

        // Full rebuild path (structure changed or fast path failed)
        // Clear layout cache and root child cache when structure changes
        self.layout_cache = None;
        self.cached_root_child = None;

        // Mark window node as dirty
        self.invalidation_tracker.mark_dirty(self.window_node);
        self.invalidation_tracker.metrics_mut().record_invalidation();
        
        if size_changed {
            // Ensure size is updated before rebuild
            self.update_window_node_size(current_width, current_height);
            self.info.size = nalgebra::Vector2::new(current_width as f64, current_height as f64);
        }
        
        self.rebuild_layout();
        
        // Clear the LAYOUT flag after rebuilding
        if self.update.get().intersects(Update::LAYOUT) {
            self.update.remove(Update::LAYOUT);
        }

        // Get the style AFTER rebuilding
        let context = LayoutContext::unbounded();
        let style = self.widget.as_ref().unwrap().layout_style(&context);
        
        let child_count = self.taffy.child_count(self.window_node);
        if child_count == 0 {
            return LayoutNode {
                layout: *self.taffy.get_final_layout(self.window_node),
                children: vec![],
            };
        }
        
        // Cache root child node ID
        let root_child = self.taffy.child_at_index(self.window_node, 0).unwrap();
        self.cached_root_child = Some(root_child);
        
        let new_layout = self
            .collect_layout(root_child, &style)
            .expect("Failed to collect layout");

        // Record performance metrics
        let layout_time = start_time.elapsed();
        self.invalidation_tracker
            .metrics_mut()
            .record_layout_time(layout_time.as_secs_f64() * 1000.0);
        self.invalidation_tracker
            .metrics_mut()
            .record_recomputation();
        
        self.invalidation_tracker.clear_all();

        new_layout
    }

    /// Rebuild the layout tree from scratch.
    fn rebuild_layout(&mut self) {
        log::debug!("Rebuilding layout tree from scratch");

        // Ensure window node size is up to date before rebuilding
        // This is important for resize events where the window size may have changed
        // but the window node style hasn't been updated yet
        let (new_width, new_height) = if let Some(window) = self.window.as_ref() {
            let size = window.inner_size();
            let scale_factor = window.scale_factor();
            let logical_size = size.to_logical::<f64>(scale_factor);
            let width = logical_size.width as u32;
            let height = logical_size.height as u32;
            self.update_window_node_size(width, height);
            self.last_window_size = (width, height);
            (width, height)
        } else if let Some(surface) = &self.surface {
            let (width, height) = surface.size();
            self.update_window_node_size(width, height);
            self.last_window_size = (width, height);
            (width, height)
        } else {
            log::warn!("No window or surface available for size reading");
            (800, 600) // Fallback
        };
        let _ = (new_width, new_height); // Suppress unused warning

        // Clear all children from the window node - this removes all existing nodes
        // Also invalidate cached root child since tree is being rebuilt
        self.cached_root_child = None;
        self.taffy
            .set_children(self.window_node, &[])
            .expect("Failed to set children");

        // Get the current style (which may have Display::None widgets)
        let context = LayoutContext::unbounded();
        let style = self.widget.as_ref().unwrap().layout_style(&context);

        // Build the layout tree (Display::None widgets will be skipped)
        self.layout_widget(self.window_node, &style)
            .expect("Failed to layout window");

        // Cache root child node ID after building tree
        if self.taffy.child_count(self.window_node) > 0 {
            if let Ok(root_child) = self.taffy.child_at_index(self.window_node, 0) {
                self.cached_root_child = Some(root_child);
            }
        }

        // Compute the layout
        self.compute_layout().expect("Failed to compute layout");
    }

    /// Check if the cursor is over any open context menu
    /// This uses the same logic as render_context_menu to ensure consistency
    fn is_cursor_over_menu(&mut self) -> bool {
        let Some(context) = self.context() else {
            // GPU context not available (e.g., during shutdown) - no menu can be open
            return false;
        };
        if !context.menu_manager.is_open() {
            return false;
        }
        
        let Some(cursor_pos) = self.info.cursor_pos else {
            return false;
        };
        
        let cursor = vello::kurbo::Point::new(cursor_pos.x, cursor_pos.y);
        let stack = context.menu_manager.get_stack();
        
        if stack.is_empty() {
            return false;
        }
        
        // Check if cursor is over any menu in the stack
        // Use the same logic as render_context_menu to ensure consistency
        for (template, position) in stack.iter() {
            use crate::menu::render::MenuGeometry;
            // Create geometry to check if cursor is in menu bounds
            let geometry = MenuGeometry::new(
                template,
                *position,
                &mut self.text_render,
                &mut self.info.font_context,
            );
            if geometry.rect.contains(cursor) {
                log::debug!(
                    "Cursor ({:.1}, {:.1}) is over menu at ({:.1}, {:.1}) with rect {:?}",
                    cursor.x, cursor.y,
                    position.x, position.y,
                    geometry.rect
                );
                return true;
            }
        }
        
        false
    }

    /// Update the widget with the current layout.
    fn update_widget(&mut self, layout_node: &LayoutNode) {
        log::debug!(
            "Updating root widget... ({} keyboard events, {} mouse buttons)",
            self.info.keys.len(),
            self.info.buttons.len()
        );
        
        // Note: cursor_pos masking is handled in update_internal() to persist through render phase
        let Some(context) = self.context() else {
            // GPU context not available (e.g., during shutdown) - skip widget update
            return;
        };
        self.update.insert(crate::tasks::block_on(self.widget.as_mut().unwrap().update(
            layout_node,
            context,
            &mut self.info,
        )));
    }

    /// Create the rendering surface.
    fn create_surface(&mut self, gpu_context: &GpuContext) {
        let platform = Platform::detect();
        log::info!("Detected platform: {:?}", platform);

        // Get window size and title
        let (width, height) = if platform == Platform::Wayland {
            // For Wayland, use configured size since window doesn't exist yet
            let size = self.config.window.size;
            (size.x as u32, size.y as u32)
        } else {
            // For Winit, use actual window size
            let window = self.window.as_ref().expect("Window should exist for Winit");
            let window_size = window.inner_size();
            (window_size.width, window_size.height)
        };
        let title = self.config.window.title.clone();

        // Create surface using platform-specific function
        self.surface = Some(
            crate::platform::create_surface_blocking(
                platform,
                self.window.clone(),
                width,
                height,
                &title,
                Some(gpu_context),
            )
            .expect("Failed to create surface"),
        );
        log::debug!("Surface created successfully");

        // Update window identity after surface creation (important for Wayland)
        // This ensures the Wayland surface protocol ID is available for menu registration
        self.update_window_identity();
    }

    /// Create the renderer from a device handle.
    fn create_renderer(&mut self, device_handle: &DeviceHandle) {
        // Build renderer options
        let options = Self::build_renderer_options(&self.config);

        // Get surface size for renderer initialization
        let (width, height) = if let Some(ref surface) = self.surface {
            surface.size()
        } else {
            let size = self.config.window.size;
            (size.x as u32, size.y as u32)
        };

        // Create renderer
        self.renderer = Some(
            crate::vgi::Renderer::new(
                &device_handle.device,
                self.config.render.backend.clone(),
                options,
                width,
                height,
            )
            .expect("Failed to create renderer"),
        );

        log::debug!("Renderer created successfully");
    }

    /// Build renderer options from configuration.
    fn build_renderer_options(config: &MayConfig) -> RendererOptions {
        RendererOptions {
            antialiasing_support: Self::convert_antialiasing_config(&config.render.antialiasing),
            num_init_threads: config.render.init_threads,
        }
    }

    /// Convert antialiasing config to support flags.
    fn convert_antialiasing_config(config: &AaConfig) -> AaSupport {
        match config {
            AaConfig::Area => AaSupport::area_only(),
            AaConfig::Msaa8 => AaSupport {
                area: false,
                msaa8: true,
                msaa16: false,
            },
            AaConfig::Msaa16 => AaSupport {
                area: false,
                msaa8: false,
                msaa16: true,
            },
        }
    }

    /// Update plugins for window event handling.
    fn update_plugins_for_window_event(
        &mut self,
        event: &mut WindowEvent,
        event_loop: &ActiveEventLoop,
    ) {
        if let (Some(window), Some(renderer), Some(surface), Some(gpu_context)) = (
            self.window.as_ref(),
            self.renderer.as_mut(),
            self.surface.as_mut(),
            self.gpu_context.as_ref(),
        ) {
            self.plugins.run(|pl| {
                pl.on_window_event(
                    event,
                    &mut self.config,
                    window,
                    renderer,
                    &mut self.scene,
                    surface,
                    &mut self.taffy,
                    self.window_node,
                    &mut self.info,
                    gpu_context,
                    &self.update,
                    &mut self.last_update,
                    event_loop,
                )
            });
        }
    }

    /// Update the window node size in the layout tree.
    /// 
    /// The window node is the root of the layout tree and should match the actual window size.
    /// Children with percent(1.0) will fill 100% of this size.
    fn update_window_node_size(&mut self, width: u32, height: u32) {
        // Set the window node to the actual window size
        // This is the root node, so it should match the AvailableSpace provided to compute_layout()
        self.taffy
            .set_style(
                self.window_node,
                Style {
                    size: Size::<Dimension> {
                        width: Dimension::length(width as f32),
                        height: Dimension::length(height as f32),
                    },
                    ..Default::default()
                },
            )
            .expect("Failed to set window node style");
    }

    /// Notify plugins that the application is resuming.
    fn notify_plugins_resume(&mut self, event_loop: &ActiveEventLoop) {
        self.plugins.run(|pl| {
            pl.on_resume(
                &mut self.config,
                &mut self.scene,
                &mut self.taffy,
                self.window_node,
                &mut self.info,
                &self.update,
                &mut self.last_update,
                event_loop,
            )
        });
    }

    #[cfg(target_os = "linux")]
    fn update_window_identity(&mut self) {
        // First, try to get identity from Wayland surface if available
        #[cfg(feature = "wayland")]
        let wayland_identity = self.surface.as_ref().and_then(|surface| {
            if let crate::vgi::Surface::Wayland(wayland_surface) = surface {
                Some(WindowIdentity::Wayland(wayland_surface.surface_key()))
            } else {
                None
            }
        });
        #[cfg(not(feature = "wayland"))]
        let wayland_identity: Option<WindowIdentity> = None;

        // If we have a Wayland identity, use it; otherwise try winit X11 window
        let identity = wayland_identity.or_else(|| {
            self.window.as_ref().and_then(|window| {
                let handle = window.window_handle().ok()?;
                match handle.as_raw() {
                    RawWindowHandle::Xlib(xlib) => Some(WindowIdentity::X11(xlib.window as u32)),
                    RawWindowHandle::Xcb(xcb) => Some(WindowIdentity::X11(xcb.window.get())),
                    _ => None,
                }
            })
        });
        self.info.set_window_identity(identity);
    }

    #[cfg(not(target_os = "linux"))]
    fn update_window_identity(&mut self) {}

    /// Create the application window.
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Creating window...");
        let window = event_loop
            .create_window(self.attrs.clone())
            .expect("Failed to create window");

        log::debug!("Window created, setting visible");
        window.set_visible(true);
        
        log::debug!("Window visible, storing Arc");
        self.window = Some(Arc::new(window));
        
        log::debug!("Updating window identity");
        self.update_window_identity();
        
        log::debug!("Window creation complete");
    }

    /// Set up the window node in the layout tree.
    fn setup_window_node(&mut self) {
        // Get window size - use surface size if available (for Wayland), otherwise use window size
        let (width, height) = if let Some(surface) = &self.surface {
            surface.size()
        } else if let Some(window) = self.window.as_ref() {
            let size = window.inner_size();
            (size.width, size.height)
        } else {
            // Fallback to configured size
            let size = self.config.window.size;
            (size.x as u32, size.y as u32)
        };

        // Set the window node to the actual window size
        // This is the root node, so it should match the AvailableSpace provided to compute_layout()
        self.taffy
            .set_style(
                self.window_node,
                Style {
                    size: Size::<Dimension> {
                        width: Dimension::length(width as f32),
                        height: Dimension::length(height as f32),
                    },
                    ..Default::default()
                },
            )
            .expect("Failed to set window node style");
    }

    /// Create the initial widget for display.
    fn create_initial_widget(&mut self) {
        self.widget = Some((self.builder)(
            AppContext::new(
                self.update.clone(),
                self.info.diagnostics.clone(),
                Arc::new(GpuContext::new().expect("Failed to create GPU context")), // Temporary GPU context
                self.info.focus_manager.clone(),
                self.menu_manager.clone(),
                self.shortcut_registry.clone(),
                self.action_callbacks.clone(),
                self.popup_manager.clone(),
                self.tooltip_request_manager.clone(),
                self.status_bar.clone(),
                self.config.settings.clone(),
            ),
            self.state.take().unwrap(),
        ));
    }

    fn process_tooltip_requests(&mut self) {
        let requests = self.tooltip_request_manager.drain_requests();
        if !requests.is_empty() {
            self.tooltip_manager.process_requests(requests);
        }
        
        // Update tooltip state based on timers
        let now = Instant::now();
        if self.tooltip_manager.update(now) {
            // Tooltip state changed - request redraw
            self.update.insert(Update::DRAW);
        }
    }

    fn process_popup_requests(&mut self, event_loop: &ActiveEventLoop) {
        let requests = self.popup_manager.drain_requests();
        if !requests.is_empty() {
            log::debug!("Processing {} popup requests", requests.len());
        } else {
            return; // No requests to process
        }
        for req in requests {
            log::debug!("Creating popup window: '{}' with size {:?} at position {:?}", 
                       req.title, req.size, req.position);

            let gpu_context = match self.gpu_context.as_ref() {
                Some(ctx) => ctx,
                None => {
                    log::error!("GPU context not initialized");
                    continue;
                },
            };

            let devices = gpu_context.enumerate_devices();
            if devices.is_empty() {
                log::error!("No GPU devices available");
                continue;
            }
            let device_handle = (self.config.render.device_selector)(devices);

            let platform = crate::platform::Platform::detect();

            let (window, mut surface) = match platform {
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                crate::platform::Platform::Wayland => {
                    let surface = crate::platform::create_surface_blocking(
                        crate::platform::Platform::Wayland,
                        None,
                        req.size.0,
                        req.size.1,
                        &req.title,
                        Some(gpu_context),
                    )
                    .expect("Failed to create Wayland surface");
                    (None, surface)
                },
                _ => {
                    let mut attrs = Window::default_attributes()
                        .with_title(req.title.clone())
                        .with_inner_size(winit::dpi::PhysicalSize::new(req.size.0, req.size.1));

                    if let Some((x, y)) = req.position {
                        attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x, y));
                    }

                    let window = match event_loop.create_window(attrs) {
                        Ok(w) => Arc::new(w),
                        Err(e) => {
                            log::error!("Failed to create popup window: {}", e);
                            continue;
                        },
                    };

                    let surface = crate::platform::create_surface_blocking(
                        crate::platform::Platform::Winit,
                        Some(window.clone()),
                        req.size.0,
                        req.size.1,
                        &req.title,
                        Some(gpu_context),
                    )
                    .expect("Failed to create Winit surface");

                    (Some(window), surface)
                },
            };

            match &mut surface {
                crate::vgi::Surface::Winit(ref mut winit_surface) => {
                    if let Err(e) = winit_surface.configure(
                        &device_handle.device,
                        &device_handle.adapter,
                        req.size.0,
                        req.size.1,
                        self.config.render.present_mode.into(),
                    ) {
                        log::error!("Failed to configure popup Winit surface: {}", e);
                        continue;
                    }
                },
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                crate::vgi::Surface::Wayland(ref mut wayland_surface) => {
                    if let Err(e) = wayland_surface.configure_surface(
                        &device_handle.device,
                        TextureFormat::Bgra8Unorm,
                        self.config.render.present_mode.into(),
                    ) {
                        log::error!("Failed to configure popup Wayland surface: {}", e);
                        continue;
                    }
                },
            }

            let renderer_options = RendererOptions {
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
            };

            let renderer = match Renderer::new(
                &device_handle.device,
                self.config.render.backend.clone(),
                renderer_options,
                req.size.0,
                req.size.1,
            ) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to create renderer for popup: {}", e);
                    continue;
                },
            };

            #[cfg(all(target_os = "linux", feature = "wayland"))]
            let (render_width, render_height, popup_scale_factor) =
                if matches!(platform, crate::platform::Platform::Wayland) {
                    (req.size.0 * 1, req.size.1 * 1, 1.0)
                } else {
                    (req.size.0, req.size.1, 1.0)
                };
            #[cfg(not(all(target_os = "linux", feature = "wayland")))]
            let (render_width, render_height, popup_scale_factor) =
                (req.size.0, req.size.1, 1.0f32);

            let mut taffy = TaffyTree::new();
            let root_node = taffy
                .new_leaf(Style {
                    size: Size {
                        width: Dimension::length(req.size.0 as f32),
                        height: Dimension::length(req.size.1 as f32),
                    },
                    ..Default::default()
                })
                .unwrap();

            let info = AppInfo {
                diagnostics: self.info.diagnostics.clone(),
                font_context: self.info.font_context.clone(),
                size: Vector2::new(req.size.0 as f64, req.size.1 as f64),
                focus_manager: self.info.focus_manager.clone(),
                ..Default::default()
            };

            let scene = Scene::new(
                self.config.render.backend.clone(),
                render_width,
                render_height,
            );

            let popup = PopupWindow {
                window: window.clone(),
                renderer,
                scene,
                surface,
                taffy,
                root_node,
                widget: req.widget,
                info,
                config: self.config.clone(),
                scale_factor: popup_scale_factor,
            };

            if let Some(ref win) = window {
                // Ensure popup window is visible (windows might be created hidden)
                win.set_visible(true);
                self.popup_windows.insert(win.id(), popup);
                win.request_redraw();
            } else {
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                {
                    let popup_id = self.wayland_popup_id_counter;
                    self.wayland_popup_id_counter = self.wayland_popup_id_counter.wrapping_add(1);
                    self.wayland_popups.insert(popup_id, popup);
                    log::debug!("Created native Wayland popup with ID: {}", popup_id);
                }
            }
        }
    }
    /// Initialize heavy components asynchronously in the background
    fn initialize_async(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Starting async initialization...");

        let platform = Platform::detect();
        log::info!("Detected platform: {:?}", platform);

        // Create surface synchronously (needed for adapter selection)
        let mut gpu_context = match GpuContext::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                log::error!("Failed to create GPU context: {}", e);
                panic!("Failed to create GPU context: {}", e);
            },
        };

        self.create_surface(&gpu_context);

        // Get adapter synchronously (needed for device creation)
        let adapter = if platform == Platform::Wayland {
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            {
                if let Some(ref mut surface) = self.surface {
                    match surface {
                        crate::vgi::Surface::Wayland(wayland_surface) => {
                            wayland_surface.wgpu_surface.as_ref().and_then(|wgpu_surface| {
                                gpu_context.request_adapter_with_surface(wgpu_surface)
                            })
                        },
                        #[cfg(target_os = "linux")]
                        crate::vgi::Surface::Winit(winit_surface) => {
                            let wgpu_surface = winit_surface.surface();
                            gpu_context.request_adapter_with_surface(wgpu_surface)
                        },
                    }
                } else {
                    None
                }
            }
            #[cfg(not(all(target_os = "linux", feature = "wayland")))]
            {
                None
            }
        } else {
            if let Some(ref mut surface) = self.surface {
                if let crate::vgi::Surface::Winit(winit_surface) = surface {
                    let wgpu_surface = winit_surface.surface();
                    gpu_context.request_adapter_with_surface(wgpu_surface)
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Create channel for async device creation
        let (tx, rx) = std::sync::mpsc::channel();
        self.gpu_init_rx = Some(rx);

        // Move items needed for device creation to async block
        let mut gpu_ctx_for_task = gpu_context;
        
        // Spawn async task
        crate::tasks::spawn(async move {
            log::debug!("Creating device (heavy operation, background)...");
            
            let result = async {
                let device_handle = match adapter {
                    Some(adapter) => gpu_ctx_for_task
                        .create_device_from_adapter(&adapter)
                        .await
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?,
                    None => gpu_ctx_for_task
                        .create_device_from_first_adapter(vello::wgpu::Backends::PRIMARY)
                        .await
                        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?,
                };
                
                gpu_ctx_for_task.add_device(device_handle.clone());
                Ok((Arc::new(gpu_ctx_for_task), device_handle))
            }.await;

            let _ = tx.send(result);
        });

        log::debug!("Async initialization started");

        if platform != Platform::Wayland {
            self.update.insert(Update::FORCE);
            if let Some(window) = &self.window {
                log::debug!("Requesting initial redraw for winit window");
                window.request_redraw();
            }
        } else {
            log::debug!("Wayland: Update flags should already be set from configure handler");
        }
    }

    fn pump_events(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(surface) = &mut self.surface {
            if surface.needs_event_dispatch() {
                match surface.dispatch_events() {
                    Ok(needs_redraw) => {
                        if needs_redraw {
                            self.update.insert(Update::DRAW);
                        }
                    },
                    Err(err) => {
                        log::info!("Surface dispatch reported close: {}", err);
                        self.handle_close_request(event_loop);
                    },
                }
            }
        }
    }

    fn update_app(&mut self, event_loop: &ActiveEventLoop) {
        // Pump events first to ensure we handle inputs/configure events immediately
        self.pump_events(event_loop);

        #[cfg(all(target_os = "linux", feature = "wayland"))]
        self.process_wayland_input_events();

        // Process popup requests first (in case they were requested during widget updates)
        self.process_popup_requests(event_loop);

        // Render native Wayland popups
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        self.render_wayland_popups();
        
        // Update window identity periodically to ensure it's set (important for Wayland)
        #[cfg(target_os = "linux")]
        {
            if self.surface.is_some() || self.window.is_some() {
                self.update_window_identity();
            }
        }

        self.update_internal(event_loop);
        
        // For X11/Winit, keep the render loop going by requesting another update
        // This is needed because about_to_wait is never called with the current winit setup
        let platform = crate::platform::Platform::detect();
        if platform == crate::platform::Platform::Winit {
            // Schedule another update to keep the loop going
            self.update.insert(Update::EVAL);
        }
    }
}

impl<W, S, F> ApplicationHandler for AppHandler<W, S, F>
where
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        log::trace!("about_to_wait called");
        
        let platform = crate::platform::Platform::detect();
        
        // Use Poll mode for all platforms to ensure continuous updates
        event_loop.set_control_flow(ControlFlow::Poll);
        
        // Drive app updates even if no winit window exists (Wayland-native path)
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        {
            if platform == crate::platform::Platform::Wayland {
                // If we have a configured surface but haven't drawn yet, force a draw so
                // the compositor receives our first buffer and maps the window.
                if let Some(ref surface) = self.surface {
                    if let crate::vgi::Surface::Wayland(ref wl) = surface {
                        if wl.has_received_configure()
                            && wl.is_configured()
                            && !wl.first_frame_seen()
                        {
                            log::debug!("about_to_wait: forcing DRAW (Wayland configured)");
                            self.update.insert(Update::FORCE | Update::DRAW);
                        }
                    }
                }
            }
        }
        
        self.update_app(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        log::trace!("user_event called");
        // Waker was called (from async thread)
        // Run update to process flags
        self.update_app(event_loop);
        
        // Keep the update loop going by requesting another redraw
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Resuming/Starting app execution...");
        
        // Set Poll mode immediately for all platforms
        event_loop.set_control_flow(ControlFlow::Poll);
        log::debug!("Set control flow to Poll");

        self.notify_plugins_resume(event_loop);

        let platform = Platform::detect();
        if platform != Platform::Wayland {
            log::debug!("Creating window for non-Wayland platform");
            self.create_window(event_loop);
            log::debug!("Window created");
        }

        log::debug!("Setting up window node");
        self.setup_window_node();
        log::debug!("Creating initial widget");
        self.create_initial_widget();
        log::debug!("Starting initialization");
        self.initialize_async(event_loop);
        log::debug!("Initialization complete, resumed() returning");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        mut event: WindowEvent,
    ) {
        log::trace!("window_event received: {:?} for window {:?}", event, window_id);
        
        self.update_plugins_for_window_event(&mut event, event_loop);

        if let Some(window) = &self.window {
            if window.id() == window_id {
                self.handle_window_event(event, event_loop);
                return;
            }
        }

        // Handle popup CloseRequested before borrowing the popup to avoid borrow conflict
        if matches!(event, WindowEvent::CloseRequested) {
            log::debug!("CloseRequested event received for window: {:?}", window_id);
            if self.popup_windows.contains_key(&window_id) {
                log::info!("Closing popup window: {:?}", window_id);
                self.popup_windows.remove(&window_id);
                return;
            }
        }

        // Debug: Log popup window events that aren't handled by main window
        if self.popup_windows.contains_key(&window_id) {
            log::debug!(
                "Popup window event: {:?} for window: {:?}",
                event,
                window_id
            );
        }

        // Check popup windows
        if let Some(popup) = self.popup_windows.get_mut(&window_id) {
            match event {
                WindowEvent::RedrawRequested => {
                    // Render popup
                    let width = popup.config.window.size.x as u32;
                    let height = popup.config.window.size.y as u32;

                    // 1. Layout
                    let context = LayoutContext::unbounded();
                    let style = popup.widget.layout_style(&context);
                    let _ = popup.taffy.set_children(popup.root_node, &[]);

                    if let Err(e) = layout_widget_tree(&mut popup.taffy, popup.root_node, &style) {
                        eprintln!("Failed to build popup layout tree: {}", e);
                    }
                    let _ = popup
                        .taffy
                        .compute_layout(popup.root_node, Size::MAX_CONTENT);

                    // 2. Render to Scene
                    let mut builder =
                        Scene::new(popup.config.render.backend.clone(), width, height);
                    {
                        let mut graphics = graphics_from_scene(&mut builder).unwrap();
                        let child_count = popup.taffy.child_count(popup.root_node);
                        if child_count > 0 {
                            let widget_node =
                                popup.taffy.child_at_index(popup.root_node, 0).unwrap();
                            match collect_layout_tree(&popup.taffy, widget_node, &style, 0.0, 0.0) {
                                Ok(layout_node) => {
                                    let context = AppContext::new(
                                        self.update.clone(),
                                        self.info.diagnostics.clone(),
                                        self.gpu_context.as_ref().unwrap().clone(),
                                        self.info.focus_manager.clone(),
                                        self.menu_manager.clone(),
                                        self.shortcut_registry.clone(),
                                        self.action_callbacks.clone(),
                                        self.popup_manager.clone(),
                                        self.tooltip_request_manager.clone(),
                                        self.status_bar.clone(),
                                        self.settings.clone(),
                                    );
                                    let theme_manager = popup.config.theme_manager.clone();
                                    theme_manager.read().unwrap().access_theme_mut(|theme| {
                                        popup.widget.render(
                                            graphics.as_mut(),
                                            theme,
                                            &layout_node,
                                            &mut popup.info,
                                            context,
                                        );
                                    });
                                },
                                Err(e) => eprintln!("Failed to collect popup layout: {}", e),
                            }
                        } else {
                            log::warn!("Popup render: No children in root node!");
                        }
                    }

                    // 3. Render to Surface
                    if let Some(gpu_context) = &self.gpu_context {
                        let devices = gpu_context.enumerate_devices();
                        if !devices.is_empty() {
                            let device_handle = (self.config.render.device_selector)(devices);

                            let render_view = match popup.surface.create_render_view(
                                &device_handle.device,
                                width,
                                height,
                            ) {
                                Ok(view) => view,
                                Err(e) => {
                                    log::error!("Failed to create render view for popup: {}", e);
                                    return;
                                },
                            };

                            if let Err(e) = popup.renderer.render_to_view(
                                &device_handle.device,
                                &device_handle.queue,
                                &builder,
                                &render_view,
                                &RenderParams {
                                    base_color: popup.config.theme_manager.read().unwrap()
                                        .access_theme(|theme| theme.window_background())
                                        .unwrap_or_else(|| vello::peniko::Color::WHITE),
                                    width,
                                    height,
                                    antialiasing_method: popup.config.render.antialiasing,
                                },
                            ) {
                                log::error!("Failed to render popup scene: {}", e);
                                return;
                            }

                            let mut encoder = device_handle.device.create_command_encoder(
                                &CommandEncoderDescriptor {
                                    label: Some("Popup Surface Blit Encoder"),
                                },
                            );

                            let surface_texture = match popup.surface.get_current_texture() {
                                Ok(t) => t,
                                Err(e) => {
                                    log::error!("Failed to get popup surface texture: {}", e);
                                    return;
                                },
                            };

                            let surface_view = surface_texture
                                .texture
                                .create_view(&TextureViewDescriptor::default());

                            if let Err(e) = popup.surface.blit_render_view(
                                &device_handle.device,
                                &mut encoder,
                                &render_view,
                                &surface_view,
                            ) {
                                log::error!("Failed to blit popup surface: {}", e);
                                return;
                            }

                            device_handle.queue.submit(Some(encoder.finish()));
                            surface_texture.present();
                        }
                    }

                    if let Some(ref win) = popup.window {
                        win.pre_present_notify();
                    }
                },
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    let physical = popup
                        .window
                        .as_ref()
                        .map(|w| w.inner_size())
                        .unwrap_or(winit::dpi::PhysicalSize::new(0, 0));
                    let _ = popup.surface.resize(physical.width, physical.height);
                    popup
                        .renderer
                        .update_render_target_size(physical.width, physical.height);

                    let logical = physical.to_logical::<f64>(scale_factor);
                    let _ = popup.taffy.set_style(
                        popup.root_node,
                        Style {
                            size: Size {
                                width: Dimension::length(logical.width as f32),
                                height: Dimension::length(logical.height as f32),
                            },
                            ..Default::default()
                        },
                    );
                    popup.config.window.size = Vector2::new(logical.width, logical.height);
                    if let Some(ref win) = popup.window {
                        win.request_redraw();
                    }
                },
                WindowEvent::Resized(size) => {
                    let _ = popup.surface.resize(size.width, size.height);
                    popup
                        .renderer
                        .update_render_target_size(size.width, size.height);

                    let scale_factor = popup
                        .window
                        .as_ref()
                        .map(|w| w.scale_factor())
                        .unwrap_or(popup.scale_factor as f64);
                    let logical = size.to_logical::<f64>(scale_factor);

                    let _ = popup.taffy.set_style(
                        popup.root_node,
                        Style {
                            size: Size {
                                width: Dimension::length(logical.width as f32),
                                height: Dimension::length(logical.height as f32),
                            },
                            ..Default::default()
                        },
                    );
                    popup.config.window.size = Vector2::new(logical.width, logical.height);
                    if let Some(ref win) = popup.window {
                        win.request_redraw();
                    }
                },
                _ => {},
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Suspending application...");

        self.window = None;
        self.surface = None;
        self.gpu_context = None;
        self.renderer = None;

        self.plugins.run(|pl| {
            pl.on_suspended(
                &mut self.config,
                &mut self.scene,
                &mut self.taffy,
                self.window_node,
                &mut self.info,
                &self.update,
                &mut self.last_update,
                event_loop,
            )
        });

        self.info.reset();
    }
}

/// Helper function to recursively build Taffy layout tree from StyleNode tree
fn layout_widget_tree(taffy: &mut TaffyTree, parent: NodeId, style: &StyleNode) -> TaffyResult<()> {
    if style.style.display == Display::None {
        return Ok(());
    }

    let node = taffy.new_leaf(style.style.clone().into())?;
    taffy.add_child(parent, node)?;

    for child in &style.children {
        if child.style.display != Display::None {
            layout_widget_tree(taffy, node, child)?;
        }
    }

    Ok(())
}

/// Helper function to recursively collect computed layout from Taffy tree
fn collect_layout_tree(
    taffy: &TaffyTree,
    node: NodeId,
    style: &StyleNode,
    parent_x: f32,
    parent_y: f32,
) -> TaffyResult<LayoutNode> {
    let mut children = Vec::new();
    let taffy_child_count = taffy.child_count(node);

    // Get the relative layout from Taffy
    let relative_layout = *taffy.get_final_layout(node);

    // Convert relative position to absolute
    let absolute_x = parent_x + relative_layout.location.x;
    let absolute_y = parent_y + relative_layout.location.y;

    // Create absolute layout
    let mut absolute_layout = relative_layout;
    absolute_layout.location.x = absolute_x;
    absolute_layout.location.y = absolute_y;

    let child_content_x = absolute_x;
    let child_content_y = absolute_y;

    let mut taffy_index = 0;
    for child_style in &style.children {
        if child_style.style.display == Display::None {
            continue;
        }

        if taffy_index < taffy_child_count {
            let child_node = taffy.child_at_index(node, taffy_index)?;
            children.push(collect_layout_tree(
                taffy,
                child_node,
                child_style,
                child_content_x,
                child_content_y,
            )?);
            taffy_index += 1;
        }
    }

    Ok(LayoutNode {
        layout: absolute_layout,
        children,
    })
}
