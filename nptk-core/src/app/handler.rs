#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod events;
mod render;
#[cfg(all(target_os = "linux", feature = "wayland"))]
mod wayland;

use crate::app::context::AppContext;
use crate::layout::LayoutNode;
use crate::vgi::graphics_from_scene;
use crate::vgi::{DeviceHandle, GpuContext};
use crate::vgi::{Renderer, RendererOptions, Scene, Surface, SurfaceTrait};
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
use nptk_theme::theme::Theme;
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
    popup_manager: crate::app::popup::PopupManager,
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
            popup_manager: crate::app::popup::PopupManager::new(),
            popup_windows: std::collections::HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_popups: std::collections::HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_popup_id_counter: 0,
            settings,
        }
    }

    /// Get the application context.
    pub fn context(&self) -> AppContext {
        AppContext::new(
            self.update.clone(),
            self.info.diagnostics.clone(),
            self.gpu_context.clone().unwrap(),
            self.info.focus_manager.clone(),
            self.menu_manager.clone(),
            self.popup_manager.clone(),
            self.settings.clone(),
        )
    }

    /// Add the parent node and its children to the layout tree.
    fn layout_widget(&mut self, parent: NodeId, style: &StyleNode) -> TaffyResult<()> {
        // If this widget itself has Display::None, don't add it to the layout tree at all
        // This ensures hidden widgets don't take up any space
        if style.style.display == Display::None {
            log::info!("Skipping widget with Display::None (parent: {:?})", parent);
            return Ok(());
        }

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

    /// Compute the layout of the root node and its children.
    fn compute_layout(&mut self) -> TaffyResult<()> {
        log::debug!("Computing root layout.");

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
        self.collect_layout_impl(node, style, 0.0, 0.0)
    }

    /// Internal implementation that accumulates parent positions.
    /// parent_x and parent_y represent the absolute position of the parent's content area (after padding).
    fn collect_layout_impl(
        &mut self,
        node: NodeId,
        style: &StyleNode,
        parent_x: f32,
        parent_y: f32,
    ) -> TaffyResult<LayoutNode> {
        let mut children = Vec::new();
        let taffy_child_count = self.taffy.child_count(node);
        let style_child_count = style.children.len();

        // Count visible children in style
        let visible_style_children: Vec<_> = style
            .children
            .iter()
            .filter(|cs| cs.style.display != Display::None)
            .collect();
        let visible_count = visible_style_children.len();

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

            // Collect this visible child from Taffy, passing our content area position as the new parent content position
            if taffy_index < taffy_child_count {
                let child_node = self.taffy.child_at_index(node, taffy_index)?;
                children.push(self.collect_layout_impl(
                    child_node,
                    child_style,
                    child_content_x,
                    child_content_y,
                )?);
                taffy_index += 1;
            } else {
                // Taffy has fewer children than expected visible children
                // This can happen if visibility changed between build and collect
                log::warn!(
                    "Taffy has fewer children ({}) than visible style children ({}), stopping collection at taffy_index {}",
                    taffy_child_count,
                    visible_count,
                    taffy_index
                );
                break;
            }
        }

        // Warn if we didn't collect all Taffy children
        if taffy_index < taffy_child_count {
            log::warn!(
                "Collected only {} children from Taffy, but Taffy has {} children",
                taffy_index,
                taffy_child_count
            );
        }

        // Log layout size and position for debugging (only for containers with children to reduce noise)
        if !children.is_empty() {
            // Log first child position to see if children are moving correctly
            let first_child_pos = if !children.is_empty() {
                format!(
                    "first_child_pos=({:.1}, {:.1})",
                    children[0].layout.location.x, children[0].layout.location.y
                )
            } else {
                "no_children".to_string()
            };
            log::info!(
                "Collected layout for node {:?}: pos=({:.1}, {:.1}), size=({:.1}, {:.1}), {} children, {} (style: {} total, {} visible, Taffy: {})",
                node,
                absolute_layout.location.x,
                absolute_layout.location.y,
                absolute_layout.size.width,
                absolute_layout.size.height,
                children.len(),
                first_child_pos,
                style_child_count,
                visible_count,
                taffy_child_count
            );
        }

        Ok(LayoutNode {
            layout: absolute_layout,
            children,
        })
    }

    /// Request a window redraw.
    fn request_redraw(&self) {
        log::debug!("Requesting redraw...");

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    /// Update the app and process events.
    /// This is called by the winit event loop periodically.
    pub fn update(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("update() called");

        // For Wayland, process events first to trigger frame callbacks
        let platform = Platform::detect();
        if platform == Platform::Wayland {
            if let Some(ref mut surface) = self.surface {
                if surface.needs_event_dispatch() {
                    match surface.dispatch_events() {
                        Ok(needs_redraw) => {
                            if needs_redraw {
                                log::debug!("Wayland events triggered redraw");
                                self.update.insert(Update::DRAW);
                            }
                        },
                        Err(err) => {
                            log::info!("Wayland surface dispatch reported close: {}", err);
                            self.update.insert(Update::EXIT);
                        },
                    }
                }
                // Fallback: if the Wayland surface has been configured, force a first draw so we attach a buffer.
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                {
                    if let crate::vgi::Surface::Wayland(ref wayland_surface) = surface {
                        // Keep scheduling redraws until the first frame callback is observed.
                        if wayland_surface.is_configured() && !wayland_surface.first_frame_seen() {
                            log::debug!(
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
        // Check for theme changes and trigger redraw if needed
        self.check_theme_changes();
        
        self.update_plugins(event_loop);

        let mut layout_node = self.ensure_layout_initialized();
        layout_node = self.update_layout_if_needed(layout_node);

        // Check if cursor is over menu for masking during render (but not during update/events)
        let original_cursor_pos = self.info.cursor_pos;
        let cursor_over_menu = self.is_cursor_over_menu();
        
        // Update widget - cursor is NOT masked here so event handling works correctly
        // Cursor will be masked during render phase only
        self.update_widget(&layout_node);

        // If widget update set LAYOUT or FORCE flags, rebuild the layout immediately
        // This ensures visibility changes take effect in the same frame
        layout_node = self.update_layout_if_needed(layout_node);

        let update_flags = self.update.get();
        log::debug!(
            "Update flags: {:?}, FORCE: {}, DRAW: {}",
            update_flags,
            update_flags.intersects(Update::FORCE),
            update_flags.intersects(Update::DRAW)
        );

        if update_flags.intersects(Update::FORCE | Update::DRAW) {
            log::info!(
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

        log::debug!("Updates per sec: {}", self.info.diagnostics.updates_per_sec);
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
    fn ensure_layout_initialized(&mut self) -> LayoutNode {
        if self.taffy.child_count(self.window_node) == 0 {
            self.setup_initial_layout();
        }

        let style = self.widget.as_ref().unwrap().layout_style();
        let child_count = self.taffy.child_count(self.window_node);
        if child_count == 0 {
            // Root widget has no children (all filtered out) - return empty layout
            log::debug!("Root widget has no children in layout tree, returning empty layout");
            return LayoutNode {
                layout: *self.taffy.get_final_layout(self.window_node),
                children: vec![],
            };
        }
        self.collect_layout(
            self.taffy.child_at_index(self.window_node, 0).unwrap(),
            &style,
        )
        .expect("Failed to collect layout")
    }

    /// Set up the initial layout tree.
    fn setup_initial_layout(&mut self) {
        log::debug!("Setting up layout...");
        let style = self.widget.as_ref().unwrap().layout_style();
        self.layout_widget(self.window_node, &style)
            .expect("Failed to layout window");
        self.compute_layout().expect("Failed to compute layout");
        self.update.insert(Update::FORCE);
    }

    /// Update layout if needed, returning the updated layout node.
    fn update_layout_if_needed(&mut self, layout_node: LayoutNode) -> LayoutNode {
        if !self.update.get().intersects(Update::LAYOUT | Update::FORCE) {
            return layout_node;
        }

        log::info!("Layout update detected! Rebuilding layout...");
        self.rebuild_layout();

        // Get the style AFTER rebuilding to ensure it matches what was built
        let style = self.widget.as_ref().unwrap().layout_style();
        let child_count = self.taffy.child_count(self.window_node);
        if child_count == 0 {
            // Root widget has no children (all filtered out) - return empty layout
            log::info!(
                "Root widget has no children in layout tree after rebuild, returning empty layout"
            );
            return LayoutNode {
                layout: *self.taffy.get_final_layout(self.window_node),
                children: vec![],
            };
        }
        let new_layout = self
            .collect_layout(
                self.taffy.child_at_index(self.window_node, 0).unwrap(),
                &style,
            )
            .expect("Failed to collect layout");

        // Log the difference in positions to verify layout is updating
        if !new_layout.children.is_empty() && !layout_node.children.is_empty() {
            let old_pos = layout_node.children[0].layout.location.y;
            let new_pos = new_layout.children[0].layout.location.y;
            if (old_pos - new_pos).abs() > 0.1 {
                log::info!(
                    "Content container moved: old_y={:.1}, new_y={:.1}, delta={:.1}",
                    old_pos,
                    new_pos,
                    new_pos - old_pos
                );
            }
        }

        new_layout
    }

    /// Rebuild the layout tree from scratch.
    fn rebuild_layout(&mut self) {
        log::info!("Rebuilding layout tree from scratch");

        // Clear all children from the window node - this removes all existing nodes
        self.taffy
            .set_children(self.window_node, &[])
            .expect("Failed to set children");

        // Get the current style (which may have Display::None widgets)
        let style = self.widget.as_ref().unwrap().layout_style();

        // Count visible children for debugging
        let visible_children: Vec<_> = style
            .children
            .iter()
            .filter(|c| c.style.display != Display::None)
            .collect();
        log::info!(
            "Rebuilding layout: root has {} total children, {} visible",
            style.children.len(),
            visible_children.len()
        );

        // Build the layout tree (Display::None widgets will be skipped)
        self.layout_widget(self.window_node, &style)
            .expect("Failed to layout window");

        // Verify the window node has the correct number of children
        let child_count = self.taffy.child_count(self.window_node);
        log::info!(
            "After layout_widget: window node has {} children (expected {})",
            child_count,
            visible_children.len()
        );

        // Compute the layout
        self.compute_layout().expect("Failed to compute layout");

        log::info!("Layout rebuild complete");
    }

    /// Check if the cursor is over any open context menu
    /// This uses the same logic as render_context_menu to ensure consistency
    fn is_cursor_over_menu(&mut self) -> bool {
        let context = self.context();
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
        let context = self.context();
        self.update.insert(self.widget.as_mut().unwrap().update(
            layout_node,
            context,
            &mut self.info,
        ));
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
    fn update_window_node_size(&mut self, width: u32, height: u32) {
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

        // Ensure window is visible
        // Windows might be created hidden, so we explicitly show them
        window.set_visible(true);

        self.window = Some(Arc::new(window));
        self.update_window_identity();
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
                self.popup_manager.clone(),
                self.config.settings.clone(),
            ),
            self.state.take().unwrap(),
        ));
    }

    fn process_popup_requests(&mut self, event_loop: &ActiveEventLoop) {
        let requests = self.popup_manager.drain_requests();
        if !requests.is_empty() {
            log::debug!("Processing {} popup requests", requests.len());
        }
        for req in requests {
            log::debug!("Creating popup window: {}", req.title);

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
    fn initialize_async(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("Starting async initialization...");

        let mut gpu_context = match GpuContext::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                log::error!("Failed to create GPU context: {}", e);
                panic!("Failed to create GPU context: {}", e);
            },
        };

        let platform = Platform::detect();
        log::info!("Detected platform: {:?}", platform);

        self.create_surface(&gpu_context);

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
            None
        };

        let device_handle = match adapter {
            Some(adapter) => gpu_context
                .create_device_from_adapter(&adapter)
                .expect("Failed to create device from adapter"),
            None => gpu_context
                .create_device_from_first_adapter(vello::wgpu::Backends::PRIMARY)
                .expect("Failed to create device from default adapter"),
        };

        gpu_context.add_device(device_handle);
        let devices = gpu_context.enumerate_devices();
        let device_handle_ref = devices.last().expect("Device should have been added");

        self.create_renderer(device_handle_ref);

        self.gpu_context = Some(Arc::new(gpu_context));
        self.async_init_complete.store(true, Ordering::Relaxed);

        log::debug!("Async initialization complete");

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
}

impl<W, S, F> ApplicationHandler for AppHandler<W, S, F>
where
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Drive app updates even if no winit window exists (Wayland-native path)
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        {
            if crate::platform::Platform::detect() == crate::platform::Platform::Wayland {
                // Always poll when running native Wayland so we can pump the custom
                // event queue even when winit has no Wayland windows to watch.
                event_loop.set_control_flow(ControlFlow::Poll);

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
        self.process_popup_requests(event_loop);

        // Render native Wayland popups
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        self.render_wayland_popups();

        self.update(event_loop);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Resuming/Starting app execution...");

        self.notify_plugins_resume(event_loop);

        // Only create winit window if not using native Wayland
        let platform = Platform::detect();
        if platform != Platform::Wayland {
            self.create_window(event_loop);
        }

        self.setup_window_node();
        self.create_initial_widget();
        self.initialize_async(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        mut event: WindowEvent,
    ) {
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
                    let style = popup.widget.layout_style();
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
                                        self.popup_manager.clone(),
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
