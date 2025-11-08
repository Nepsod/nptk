use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use nalgebra::Vector2;
use taffy::{
    AvailableSpace, Dimension, NodeId, PrintTree, Size, Style, TaffyResult, TaffyTree,
    TraversePartialTree,
};
use vello::util::RenderContext;
use vello::{AaConfig, AaSupport, RenderParams};
use crate::vgi::{Renderer, Scene, RendererOptions, Surface, Platform, SurfaceTrait};
use crate::vgi::graphics_from_scene;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::context::AppContext;
use crate::app::font_ctx::FontContext;
use crate::app::info::AppInfo;
use crate::app::update::{Update, UpdateManager};
use crate::config::MayConfig;
use crate::layout::{LayoutNode, StyleNode};
use crate::plugin::PluginManager;
use crate::widget::Widget;
use nptk_theme::theme::Theme;

/// The core application handler. You should use [MayApp](crate::app::MayApp) instead for running applications.
pub struct AppHandler<T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    config: MayConfig<T>,
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
    render_ctx: Option<Arc<RenderContext>>,
    update: UpdateManager,
    last_update: Instant,
    plugins: PluginManager<T>,
    /// Tracks whether async initialization is complete
    async_init_complete: Arc<AtomicBool>,
}

impl<T, W, S, F> AppHandler<T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    /// Create a new handler with given window attributes, config, widget and state.
    pub fn new(
        attrs: WindowAttributes,
        config: MayConfig<T>,
        builder: F,
        state: S,
        font_context: FontContext,
        update: UpdateManager,
        plugins: PluginManager<T>,
    ) -> Self {
        let mut taffy = TaffyTree::with_capacity(16);

        // gets configured on resume
        let window_node = taffy
            .new_leaf(Style::default())
            .expect("Failed to create window node");

        let size = config.window.size;
        let backend = config.render.backend.clone();

        Self {
            attrs,
            window: None,
            renderer: None,
            config,
            scene: Scene::new(backend, 0, 0), // Will be updated on resize
            surface: None,
            taffy,
            widget: None,
            info: AppInfo {
                font_context,
                size,
                ..Default::default()
            },
            window_node,
            builder,
            state: Some(state),
            render_ctx: None,
            update,
            last_update: Instant::now(),
            plugins,
            async_init_complete: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the application context.
    pub fn context(&self) -> AppContext {
        AppContext::new(
            self.update.clone(),
            self.info.diagnostics,
            self.render_ctx.clone().unwrap(),
            self.info.focus_manager.clone(),
        )
    }

    /// Add the parent node and its children to the layout tree.
    fn layout_widget(&mut self, parent: NodeId, style: &StyleNode) -> TaffyResult<()> {
        log::debug!("Laying out widget: {:?}", parent);

        let node = self.taffy.new_leaf(style.style.clone().into())?;

        self.taffy.add_child(parent, node)?;

        for child in &style.children {
            self.layout_widget(node, child)?;
        }

        Ok(())
    }

    /// Compute the layout of the root node and its children.
    fn compute_layout(&mut self) -> TaffyResult<()> {
        log::debug!("Computing root layout.");

        self.taffy.compute_layout(
            self.window_node,
            Size::<AvailableSpace> {
                width: AvailableSpace::Definite(
                    self.window.as_ref().unwrap().inner_size().width as f32,
                ),
                height: AvailableSpace::Definite(
                    self.window.as_ref().unwrap().inner_size().height as f32,
                ),
            },
        )?;
        Ok(())
    }

    /// Collect the computed layout of the given node and its children. Make sure to call [AppHandler::compute_layout] before, to not get dirty results.
    fn collect_layout(&mut self, node: NodeId, style: &StyleNode) -> TaffyResult<LayoutNode> {
        log::debug!("Collecting layout for node: {:?}", node);

        let mut children = Vec::with_capacity(style.children.capacity());

        for (i, child) in style.children.iter().enumerate() {
            children.push(self.collect_layout(self.taffy.child_at_index(node, i)?, child)?);
        }

        Ok(LayoutNode {
            layout: *self.taffy.get_final_layout(node),
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
    fn update(&mut self, event_loop: &ActiveEventLoop) {
        self.update_plugins(event_loop);
        
        let mut layout_node = self.ensure_layout_initialized();
        layout_node = self.update_layout_if_needed(layout_node);
        
        self.update_widget(&layout_node);
        
        if self.update.get().intersects(Update::FORCE | Update::DRAW) {
            self.render_frame(&layout_node, event_loop);
        }

        self.handle_update_flags(event_loop);
        self.update_diagnostics();
    }

    /// Update plugins with current state.
    fn update_plugins(&mut self, event_loop: &ActiveEventLoop) {
        self.plugins.run(|pl| {
            pl.on_update(
                &mut self.config,
                self.window.as_ref().expect("Window not initialized"),
                self.renderer.as_mut().expect("Renderer not initialized"),
                &mut self.scene,
                self.surface.as_mut().expect("Surface not initialized"),
                &mut self.taffy,
                self.window_node,
                &mut self.info,
                self.render_ctx
                    .as_mut()
                    .expect("Render context not initialized"),
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

        log::debug!("Layout update detected!");
        self.rebuild_layout();
        
        let style = self.widget.as_ref().unwrap().layout_style();
        self.collect_layout(
            self.taffy.child_at_index(self.window_node, 0).unwrap(),
            &style,
        )
        .expect("Failed to collect layout")
    }

    /// Rebuild the layout tree from scratch.
    fn rebuild_layout(&mut self) {
        self.taffy
            .set_children(self.window_node, &[])
            .expect("Failed to set children");
        
        let style = self.widget.as_ref().unwrap().layout_style();
        self.layout_widget(self.window_node, &style)
            .expect("Failed to layout window");
        self.compute_layout().expect("Failed to compute layout");
    }

    /// Update the widget with the current layout.
    fn update_widget(&mut self, layout_node: &LayoutNode) {
        log::debug!("Updating root widget...");
        let context = self.context();
        self.update.insert(
            self.widget
                .as_mut()
                .unwrap()
                .update(layout_node, context, &mut self.info),
        );
    }

    /// Render a frame to the screen.
    fn render_frame(&mut self, layout_node: &LayoutNode, event_loop: &ActiveEventLoop) {
        log::debug!("Draw update detected!");
        let render_start = Instant::now();

        self.scene.reset();
        let scene_reset_time = render_start.elapsed();

        let widget_render_time = self.render_widget(layout_node);
        let postfix_render_time = self.render_postfix(layout_node);

        if let Some(render_times) = self.render_to_surface(render_start, scene_reset_time, widget_render_time, postfix_render_time, event_loop) {
            self.print_render_profile(render_times);
        }
    }

    /// Render the main widget content.
    fn render_widget(&mut self, layout_node: &LayoutNode) -> Duration {
        log::debug!("Rendering root widget...");
        let start = Instant::now();
        
        let context = self.context();
        // Use unified Graphics API that works with both Vello and Hybrid backends
        let mut graphics = graphics_from_scene(&mut self.scene)
            .expect("Failed to create graphics from scene");
        self.widget.as_mut().unwrap().render(
            graphics.as_mut(),
            &mut self.config.theme,
            layout_node,
            &mut self.info,
            context,
        );
        
        start.elapsed()
    }

    /// Render postfix content (overlays, popups).
    fn render_postfix(&mut self, layout_node: &LayoutNode) -> Duration {
        log::debug!("Rendering postfix content...");
        let start = Instant::now();
        
        let context = self.context();
        // Use unified Graphics API that works with both Vello and Hybrid backends
        let mut graphics = graphics_from_scene(&mut self.scene)
            .expect("Failed to create graphics from scene");
        self.widget.as_mut().unwrap().render_postfix(
            graphics.as_mut(),
            &mut self.config.theme,
            layout_node,
            &mut self.info,
            context,
        );
        
        start.elapsed()
    }

    /// Render the scene to the surface, returning render times if successful.
    fn render_to_surface(
        &mut self,
        render_start: Instant,
        scene_reset_time: Duration,
        widget_render_time: Duration,
        postfix_render_time: Duration,
        event_loop: &ActiveEventLoop,
    ) -> Option<RenderTimes> {
        let renderer = self.renderer.as_mut()?;
        let render_ctx = self.render_ctx.as_ref()?;
        let surface = self.surface.as_mut()?;

        let device_handle = render_ctx.devices.first()?;

        // Get window size from surface (works for both Winit and Wayland)
        let (width, height) = surface.size();
        if width == 0 || height == 0 {
            log::debug!("Surface invalid. Skipping render.");
            return None;
        }

        // Dispatch Wayland events if needed and check for close request
        let should_close = if surface.needs_event_dispatch() {
            if let Ok(needs_redraw) = surface.dispatch_events() {
                if needs_redraw {
                    self.update.insert(Update::DRAW);
                }
            }
            
            // Check if Wayland window requested close (before mutable borrow)
            if let crate::vgi::Surface::Wayland(wayland_surface) = &*surface {
                wayland_surface.should_close()
            } else {
                false
            }
        } else {
            false
        };
        
        // Handle close request if needed
        if should_close {
            self.handle_close_request(event_loop);
            return None; // Don't render if closing
        }

        let surface_get_start = Instant::now();
        let surface_texture = surface
            .get_current_texture()
            .expect("Failed to get surface texture");
        let surface_get_time = surface_get_start.elapsed();

        // Pre-present notification (only for winit windows)
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }

        let gpu_render_start = Instant::now();
        renderer
            .render_to_surface(
                &device_handle.device,
                &device_handle.queue,
                &self.scene,
                &surface_texture,
                &RenderParams {
                    base_color: self.config.theme.window_background(),
                    width,
                    height,
                    antialiasing_method: self.config.render.antialiasing,
                },
            )
            .expect("Failed to render to surface");
        let gpu_render_time = gpu_render_start.elapsed();

        let present_start = Instant::now();
        surface.present().expect("Failed to present surface");
        let present_time = present_start.elapsed();

        Some(RenderTimes {
            scene_reset_time,
            widget_render_time,
            postfix_render_time,
            surface_get_time,
            gpu_render_time,
            present_time,
            total_time: render_start.elapsed(),
        })
    }

    /// Print render profiling information if enabled.
    fn print_render_profile(&self, times: RenderTimes) {
        if std::env::var("NPTK_PROFILE").is_ok() {
            eprintln!(
                "[NPTK Profile] Scene reset: {:.2}ms | Widget render: {:.2}ms | Postfix: {:.2}ms | Surface get: {:.2}ms | GPU render: {:.2}ms | Present: {:.2}ms | Total: {:.2}ms",
                times.scene_reset_time.as_secs_f64() * 1000.0,
                times.widget_render_time.as_secs_f64() * 1000.0,
                times.postfix_render_time.as_secs_f64() * 1000.0,
                times.surface_get_time.as_secs_f64() * 1000.0,
                times.gpu_render_time.as_secs_f64() * 1000.0,
                times.present_time.as_secs_f64() * 1000.0,
                times.total_time.as_secs_f64() * 1000.0
            );
        }
    }

    /// Handle update flags (eval, exit).
    fn handle_update_flags(&mut self, event_loop: &ActiveEventLoop) {
        if self.update.get().intersects(Update::EVAL | Update::FORCE) {
            log::debug!("Evaluation update detected!");
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }

        if self.update.get().intersects(Update::EXIT) {
            event_loop.exit();
            return;
        }

        self.info.reset();
        self.update.clear();
    }

    /// Update diagnostics counters.
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
}

/// Render timing information for profiling.
struct RenderTimes {
    scene_reset_time: Duration,
    widget_render_time: Duration,
    postfix_render_time: Duration,
    surface_get_time: Duration,
    gpu_render_time: Duration,
    present_time: Duration,
    total_time: Duration,
}

impl<T, W, S, F> AppHandler<T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{

    /// Initialize heavy components asynchronously in the background
    fn initialize_async(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("Starting async initialization...");
        
        // Detect platform early
        let platform = Platform::detect();
        
        // For Wayland, we need to create the surface first so RenderContext can enumerate
        // adapters with Wayland compatibility. The issue is that RenderContext::new() 
        // enumerates adapters, but without a Wayland surface, it can't test compatibility.
        // So we create a temporary Wayland surface first, then create RenderContext,
        // then recreate the surface properly with wgpu support.
        if platform == Platform::Wayland {
            log::debug!("Wayland detected: creating temporary surface for adapter enumeration");
            // Create temporary Wayland surface (without wgpu surface) to help with adapter enumeration
            let size = self.config.window.size;
            let title = self.config.window.title.clone();
            if let Ok(wayland_surface) = crate::vgi::wayland_surface::WaylandSurface::new(
                size.x as u32,
                size.y as u32,
                &title,
                None, // No render context yet
            ) {
                self.surface = Some(crate::vgi::Surface::Wayland(wayland_surface));
            }
        }
        
        let mut render_ctx = Self::create_render_context();
        self.create_surface(&mut render_ctx);
        self.create_renderer(&render_ctx);

        self.render_ctx = Some(Arc::new(render_ctx));
        self.update.set(Update::FORCE);
        self.async_init_complete.store(true, Ordering::Relaxed);
        
        log::debug!("Async initialization complete");
    }

    /// Create a new render context.
    fn create_render_context() -> RenderContext {
        log::debug!("Creating render context...");
        let ctx = RenderContext::new();
        log::debug!("Render context created successfully");
        ctx
    }

    /// Create the rendering surface.
    fn create_surface(&mut self, render_ctx: &mut RenderContext) {
        let window = match &self.window {
            Some(w) => w.clone(),
            None => {
                log::error!("Window not available during surface creation");
                return;
            }
        };

        log::debug!("Creating surface...");
        
        // Detect platform
        let platform = Platform::detect();
        log::info!("Detected platform: {:?}", platform);
        
        // Get window size (for Wayland, we'll use the configured size)
        let (width, height) = if platform == Platform::Wayland {
            // For Wayland, use configured size since window doesn't exist yet
            let size = self.config.window.size;
            (size.x as u32, size.y as u32)
        } else {
            // For Winit, use actual window size
            let window_size = window.inner_size();
            (window_size.width, window_size.height)
        };
        let title = self.config.window.title.clone();
        
        // For Wayland, if we already have a temporary surface, replace it with one that has wgpu surface
        if platform == Platform::Wayland && matches!(self.surface, Some(crate::vgi::Surface::Wayland(_))) {
            log::debug!("Replacing temporary Wayland surface with one that has wgpu surface");
            // Drop the old temporary surface first
            drop(self.surface.take());
        }
        
        self.surface = Some(
            crate::vgi::platform::create_surface_blocking(
                platform,
                Some(window),
                width,
                height,
                &title,
                Some(render_ctx),
            )
            .expect("Failed to create surface"),
        );
        log::debug!("Surface created successfully");
    }

    /// Create the renderer with the given render context.
    fn create_renderer(&mut self, render_ctx: &RenderContext) {
        log::debug!("Requesting device handle via selector...");
        log::debug!("Available devices: {}", render_ctx.devices.len());
        
        if render_ctx.devices.is_empty() {
            log::error!("No GPU devices found. This may be a Wayland compatibility issue.");
            log::error!("Try running with NPTK_USE_NATIVE_WAYLAND=0 to use Winit-based rendering.");
            panic!("No devices found - cannot create renderer. See logs for details.");
        }
        
        let device_handle = (self.config.render.device_selector)(&render_ctx.devices);

        log::debug!("Creating renderer...");
        if self.config.render.cpu {
            eprintln!("[NPTK] Renderer configured with CPU path processing enabled");
            log::info!("Renderer configured with CPU path processing enabled");
        }
        
        // Get window size for Hybrid renderer (needs width/height for RenderTargetConfig)
        // Use surface size if available, otherwise fall back to window or default
        let (width, height) = if let Some(surface) = &self.surface {
            let (w, h) = surface.size();
            (w, h)
        } else if let Some(window) = self.window.as_ref() {
            let size = window.inner_size();
            (size.width, size.height)
        } else {
            (1920, 1080) // Default size if neither surface nor window available
        };
        
        // Note: Hybrid backend is disabled due to wgpu version conflict,
        // so scene recreation is not needed (Hybrid falls back to Vello)
        
        // Get surface format for renderer options
        let surface_format = self.surface.as_ref()
            .map(|s| s.format())
            .unwrap_or(vello::wgpu::TextureFormat::Bgra8Unorm);
        
        self.renderer = Some(
            Renderer::new(
                &device_handle.device,
                self.config.render.backend.clone(),
                Self::build_renderer_options(&self.config, &surface_format),
                width,
                height,
            )
            .expect("Failed to create renderer"),
        );
    }

    /// Build renderer options from configuration.
    fn build_renderer_options(config: &MayConfig<T>, surface_format: &vello::wgpu::TextureFormat) -> RendererOptions {
        RendererOptions {
            surface_format: Some(*surface_format),
            use_cpu: config.render.cpu,
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
    fn update_plugins_for_window_event(&mut self, event: &mut WindowEvent, event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(renderer), Some(surface), Some(render_ctx)) = (
            self.window.as_ref(),
            self.renderer.as_mut(),
            self.surface.as_mut(),
            self.render_ctx.as_ref(),
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
                    render_ctx,
                    &self.update,
                    &mut self.last_update,
                    event_loop,
                )
            });
        }
    }

    /// Handle a window event.
    fn handle_window_event(&mut self, event: WindowEvent, event_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::Resized(new_size) => self.handle_resize(new_size, event_loop),
            WindowEvent::CloseRequested => self.handle_close_request(event_loop),
            WindowEvent::RedrawRequested => self.update(event_loop),
            WindowEvent::CursorLeft { .. } => {
                self.info.cursor_pos = None;
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.info.cursor_pos = Some(Vector2::new(position.x, position.y));
                self.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.info.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, device_id, is_synthetic } => {
                self.handle_keyboard_input(event, device_id, is_synthetic);
            }
            WindowEvent::MouseInput { device_id, button, state } => {
                self.handle_mouse_input(device_id, button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.info.mouse_scroll_delta = Some(delta);
                self.request_redraw();
            }
            WindowEvent::Ime(ime_event) => {
                self.info.ime_events.push(ime_event);
                self.request_redraw();
            }
            WindowEvent::Destroyed => log::info!("Window destroyed! Exiting..."),
            _ => (),
        }
    }

    /// Handle window resize event.
    fn handle_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, _event_loop: &ActiveEventLoop) {
        if new_size.width == 0 || new_size.height == 0 {
            log::debug!("Window size is 0x0, ignoring resize event.");
            return;
        }

        log::info!("Window resized to {}x{}", new_size.width, new_size.height);

        if let Some(surface) = &mut self.surface {
            // Resize the surface using the SurfaceTrait
            if let Err(e) = surface.resize(new_size.width, new_size.height) {
                log::error!("Failed to resize surface: {}", e);
            }
        }

        // Note: Hybrid backend is disabled due to wgpu version conflict,
        // so scene recreation is not needed (Hybrid falls back to Vello)

        self.update_window_node_size(new_size.width, new_size.height);
        self.info.size = Vector2::new(new_size.width as f64, new_size.height as f64);
        self.request_redraw();
        self.update.insert(Update::DRAW | Update::LAYOUT);
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

    /// Handle window close request.
    fn handle_close_request(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Window Close requested...");
        log::debug!("Cleaning up resources...");

        if self.config.window.close_on_request {
            event_loop.exit();
        }
    }

    /// Handle keyboard input event.
    fn handle_keyboard_input(
        &mut self,
        event: winit::event::KeyEvent,
        device_id: winit::event::DeviceId,
        is_synthetic: bool,
    ) {
        if is_synthetic {
            return;
        }

        if event.state == ElementState::Pressed {
            use winit::keyboard::{KeyCode, PhysicalKey};
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Tab) => {
                    self.handle_tab_navigation();
                    self.request_redraw();
                    return;
                }
                PhysicalKey::Code(KeyCode::Escape) => {
                    // Handle ESC key for modal overlays
                }
                _ => {}
            }
        }

        self.info.keys.push((device_id, event));
        self.request_redraw();
    }

    /// Handle tab navigation for focus management.
    fn handle_tab_navigation(&mut self) {
        if let Ok(mut manager) = self.info.focus_manager.lock() {
            if self.info.modifiers.shift_key() {
                manager.focus_previous();
            } else {
                manager.focus_next();
            }
            self.update.insert(Update::FOCUS | Update::DRAW);
        }
    }

    /// Handle mouse input event.
    fn handle_mouse_input(
        &mut self,
        device_id: winit::event::DeviceId,
        button: MouseButton,
        state: ElementState,
    ) {
        if button == MouseButton::Left && state == ElementState::Pressed {
            if let Some(cursor_pos) = self.info.cursor_pos {
                if let Ok(mut manager) = self.info.focus_manager.lock() {
                    if manager.handle_click(cursor_pos.x, cursor_pos.y) {
                        self.update.insert(Update::FOCUS | Update::DRAW);
                    }
                }
            }
        }

        self.info.buttons.push((device_id, button, state));
        self.request_redraw();
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

    /// Create the application window.
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Creating window...");
        self.window = Some(Arc::new(
            event_loop
                .create_window(self.attrs.clone())
                .expect("Failed to create window"),
        ));
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
                self.info.diagnostics,
                Arc::new(RenderContext::new()), // Temporary render context
                self.info.focus_manager.clone(),
            ),
            self.state.take().unwrap(),
        ));
    }
}

impl<T, W, S, F> ApplicationHandler for AppHandler<T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Resuming/Starting app execution...");

        self.notify_plugins_resume(event_loop);
        self.create_window(event_loop);
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
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Suspending application...");

        self.window = None;
        self.surface = None;
        self.render_ctx = None;
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
