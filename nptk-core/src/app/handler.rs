use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use nalgebra::Vector2;
use taffy::{
    AvailableSpace, Dimension, NodeId, PrintTree, Size, Style, TaffyResult, TaffyTree,
    TraversePartialTree,
};
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, AaSupport, RenderParams, Scene};
use crate::renderer::{UnifiedRenderer, RendererOptions as UnifiedRendererOptions};
use crate::vgi::vello_vg::VelloGraphics;
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
pub struct AppHandler<'a, T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    config: MayConfig<T>,
    attrs: WindowAttributes,
    window: Option<Arc<Window>>,
    renderer: Option<UnifiedRenderer>,
    scene: Scene,
    surface: Option<RenderSurface<'a>>,
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

impl<T, W, S, F> AppHandler<'_, T, W, S, F>
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

        Self {
            attrs,
            window: None,
            renderer: None,
            config,
            scene: Scene::new(),
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
        // update plugins
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

        // completely layout widgets if taffy is not set up yet (e.g. during first update)
        if self.taffy.child_count(self.window_node) == 0 {
            log::debug!("Setting up layout...");

            let style = self.widget.as_ref().unwrap().layout_style();

            self.layout_widget(self.window_node, &style)
                .expect("Failed to layout window");

            self.compute_layout().expect("Failed to compute layout");

            self.update.insert(Update::FORCE);
        }

        let style = self.widget.as_ref().unwrap().layout_style();

        let mut layout_node = self
            .collect_layout(
                self.taffy.child_at_index(self.window_node, 0).unwrap(),
                &style,
            )
            .expect("Failed to collect layout");

        // update call to check if app should re-evaluate
        log::debug!("Updating root widget...");
        let context = self.context();
        self.update.insert(
            self.widget
                .as_mut()
                .unwrap()
                .update(&layout_node, context, &mut self.info),
        );

        // check if app should re-evaluate layout
        if self.update.get().intersects(Update::LAYOUT | Update::FORCE) {
            log::debug!("Layout update detected!");

            // clear all nodes (except root window node)
            self.taffy
                .set_children(self.window_node, &[])
                .expect("Failed to set children");

            let style = self.widget.as_ref().unwrap().layout_style();

            self.layout_widget(self.window_node, &style)
                .expect("Failed to layout window");

            self.compute_layout().expect("Failed to compute layout");

            layout_node = self
                .collect_layout(
                    self.taffy.child_at_index(self.window_node, 0).unwrap(),
                    &style,
                )
                .expect("Failed to collect layout");
        }

        // check if app should redraw
        if self.update.get().intersects(Update::FORCE | Update::DRAW) {
            log::debug!("Draw update detected!");

            let render_start = Instant::now();

            // clear scene
            self.scene.reset();
            let scene_reset_time = render_start.elapsed();

            let context = self.context();

            log::debug!("Rendering root widget...");
            let widget_render_start = Instant::now();
            let mut graphics = VelloGraphics::new(&mut self.scene);
            self.widget.as_mut().unwrap().render(
                &mut graphics,
                &mut self.config.theme,
                &layout_node,
                &mut self.info,
                context,
            );
            let widget_render_time = widget_render_start.elapsed();

            // Render postfix content (overlays, popups) after main content
            log::debug!("Rendering postfix content...");
            let postfix_render_start = Instant::now();
            let context = self.context();
            let mut graphics = VelloGraphics::new(&mut self.scene);
            self.widget.as_mut().unwrap().render_postfix(
                &mut graphics,
                &mut self.config.theme,
                &layout_node,
                &mut self.info,
                context,
            );
            let postfix_render_time = postfix_render_start.elapsed();

            // Only render if all resources are available
            if let (Some(renderer), Some(render_ctx), Some(surface), Some(window)) = (
                self.renderer.as_mut(),
                self.render_ctx.as_ref(),
                self.surface.as_ref(),
                self.window.as_ref(),
            ) {
                let device_handle = match render_ctx.devices.first() {
                    Some(handle) => handle,
                    None => {
                        log::warn!("No devices available, skipping render");
                        return;
                    }
                };

            // check surface validity
            if window.inner_size().width != 0 && window.inner_size().height != 0 {
                let surface_get_start = Instant::now();
                let surface_texture = surface
                    .surface
                    .get_current_texture()
                    .expect("Failed to get surface texture");
                let surface_get_time = surface_get_start.elapsed();

                // make sure winit knows that the surface texture is ready to be presented
                window.pre_present_notify();

                // TODO: this panics if canvas didn't change (no operation was done) in debug mode
                let gpu_render_start = Instant::now();
                renderer
                    .render_to_surface(
                        &device_handle.device,
                        &device_handle.queue,
                        &self.scene,
                        &surface_texture,
                        &RenderParams {
                            base_color: self.config.theme.window_background(),
                            width: window.inner_size().width,
                            height: window.inner_size().height,
                            antialiasing_method: self.config.render.antialiasing,
                        },
                    )
                    .expect("Failed to render to surface");
                let gpu_render_time = gpu_render_start.elapsed();

                let present_start = Instant::now();
                surface_texture.present();
                let present_time = present_start.elapsed();

                let total_render_time = render_start.elapsed();
                
                // Only print profiling if NPTK_PROFILE is set (to avoid spam)
                // Useful for debugging performance issues, especially with Vello rendering backend
                if std::env::var("NPTK_PROFILE").is_ok() {
                    eprintln!(
                        "[NPTK Profile] Scene reset: {:.2}ms | Widget render: {:.2}ms | Postfix: {:.2}ms | Surface get: {:.2}ms | GPU render: {:.2}ms | Present: {:.2}ms | Total: {:.2}ms",
                        scene_reset_time.as_secs_f64() * 1000.0,
                        widget_render_time.as_secs_f64() * 1000.0,
                        postfix_render_time.as_secs_f64() * 1000.0,
                        surface_get_time.as_secs_f64() * 1000.0,
                        gpu_render_time.as_secs_f64() * 1000.0,
                        present_time.as_secs_f64() * 1000.0,
                        total_render_time.as_secs_f64() * 1000.0
                    );
                }
            } else {
                log::debug!("Surface invalid. Skipping render.");
            }
            } // Close the resource availability check
        }

        // check if app should re-evaluate
        if self.update.get().intersects(Update::EVAL | Update::FORCE) {
            log::debug!("Evaluation update detected!");

            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
        // update the app if requested
        if self.update.get().intersects(Update::EXIT) {
            event_loop.exit();
            return;
        }

        // reset AppInfo and update states
        self.info.reset();
        self.update.clear();

        // update diagnostics
        if self.last_update.elapsed() >= Duration::from_secs(1) {
            self.last_update = Instant::now();

            // calc avg updates per sec through updates per sec NOW divided by 2
            self.info.diagnostics.updates_per_sec =
                (self.info.diagnostics.updates_per_sec + self.info.diagnostics.updates) / 2;

            // reset current updates per seconds
            self.info.diagnostics.updates = 0;
        } else {
            // increase updates per sec NOW by 1
            self.info.diagnostics.updates += 1;
        }

        log::debug!("Updates per sec: {}", self.info.diagnostics.updates_per_sec);
    }
}

impl<T, W, S, F> AppHandler<'_, T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{

    /// Initialize heavy components asynchronously in the background
    fn initialize_async(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("Starting async initialization...");
        
        // For now, we'll use a simplified approach that defers only the most expensive operations
        // The full async approach would require significant architectural changes
        
        // Create render context immediately (this is relatively fast)
        log::debug!("Creating render context...");
        let mut render_ctx = RenderContext::new();
        log::debug!("Render context created successfully");
        
        // Create surface with a timeout to avoid blocking too long
        if let Some(window) = &self.window {
            log::debug!("Creating surface...");
            self.surface = Some(
                crate::tasks::block_on(async {
                    log::debug!("Starting surface creation...");
                    let result = render_ctx
                        .create_surface(
                            window.clone(),
                            window.inner_size().width,
                            window.inner_size().height,
                            self.config.render.present_mode,
                        )
                        .await;
                    log::debug!("Surface creation completed");
                    result
                })
                .expect("Failed to create surface"),
            );
            log::debug!("Surface created successfully");
        } else {
            log::error!("Window not available during surface creation");
            return;
        }

        // Create renderer (this can be slow)
        log::debug!("Requesting device handle via selector...");
        let device_handle = (self.config.render.device_selector)(&render_ctx.devices);

        log::debug!("Creating renderer...");
        if self.config.render.cpu {
            eprintln!("[NPTK] Renderer configured with CPU path processing enabled");
            log::info!("Renderer configured with CPU path processing enabled");
        }
        self.renderer = Some(
            UnifiedRenderer::new(
                &device_handle.device,
                self.config.render.backend,
                UnifiedRendererOptions {
                    surface_format: Some(self.surface.as_ref().unwrap().format),
                    use_cpu: self.config.render.cpu,
                    antialiasing_support: match self.config.render.antialiasing {
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
                    },
                    num_init_threads: self.config.render.init_threads,
                },
            )
            .expect("Failed to create renderer"),
        );

        self.render_ctx = Some(Arc::new(render_ctx));
        self.update.set(Update::FORCE);
        self.async_init_complete.store(true, Ordering::Relaxed);
        
        log::debug!("Async initialization complete");
    }
}

impl<T, W, S, F> ApplicationHandler for AppHandler<'_, T, W, S, F>
where
    T: Theme,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Resuming/Starting app execution...");

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

        // Create window immediately for fast startup
        log::debug!("Creating window...");
        self.window = Some(Arc::new(
            event_loop
                .create_window(self.attrs.clone())
                .expect("Failed to create window"),
        ));

        // Set window style immediately
        self.taffy
            .set_style(
                self.window_node,
                Style {
                    size: Size::<Dimension> {
                        width: Dimension::length(
                            self.window.as_ref().unwrap().inner_size().width as f32,
                        ),
                        height: Dimension::length(
                            self.window.as_ref().unwrap().inner_size().height as f32,
                        ),
                    },
                    ..Default::default()
                },
            )
            .expect("Failed to set window node style");

        // Create a basic widget immediately for display
        self.widget = Some((self.builder)(
            AppContext::new(
                self.update.clone(),
                self.info.diagnostics,
                Arc::new(RenderContext::new()), // Temporary render context
                self.info.focus_manager.clone(),
            ),
            self.state.take().unwrap(),
        ));

        // Initialize heavy components asynchronously
        self.initialize_async(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        mut event: WindowEvent,
    ) {
        // Only run plugins if all resources are still available
        if let (Some(window), Some(renderer), Some(surface), Some(render_ctx)) = (
            self.window.as_ref(),
            self.renderer.as_mut(),
            self.surface.as_mut(),
            self.render_ctx.as_ref(),
        ) {
            self.plugins.run(|pl| {
                pl.on_window_event(
                    &mut event,
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

        if let Some(window) = &self.window {
            if window.id() == window_id {
                match event {
                    WindowEvent::Resized(new_size) => {
                        if new_size.width != 0 && new_size.height != 0 {
                            log::info!("Window resized to {}x{}", new_size.width, new_size.height);

                            if let Some(ctx) = &self.render_ctx {
                                if let Some(surface) = &mut self.surface {
                                    ctx.resize_surface(surface, new_size.width, new_size.height);
                                }
                            }

                            self.taffy
                                .set_style(
                                    self.window_node,
                                    Style {
                                        size: Size::<Dimension> {
                                            width: Dimension::length(new_size.width as f32),
                                            height: Dimension::length(new_size.height as f32),
                                        },
                                        ..Default::default()
                                    },
                                )
                                .expect("Failed to set window node style");

                            self.info.size =
                                Vector2::new(new_size.width as f64, new_size.height as f64);

                            // Update overlay positions for new screen size

                            self.request_redraw();

                            self.update.insert(Update::DRAW | Update::LAYOUT);
                        } else {
                            log::debug!("Window size is 0x0, ignoring resize event.");
                        }
                    },

                    WindowEvent::CloseRequested => {
                        log::info!("Window Close requested...");

                        // Note: Devices will be automatically cleaned up when render_ctx Arc is dropped
                        // Manual destruction is not needed and causes segfaults due to Arc immutability
                        log::debug!("Cleaning up resources...");

                        if self.config.window.close_on_request {
                            event_loop.exit();
                        }
                    },

                    WindowEvent::RedrawRequested => {
                        self.update(event_loop);
                    },

                    WindowEvent::CursorLeft { .. } => {
                        self.info.cursor_pos = None;
                        self.request_redraw();
                    },

                    WindowEvent::CursorMoved { position, .. } => {
                        self.info.cursor_pos = Some(Vector2::new(position.x, position.y));
                        self.request_redraw();
                    },

                    WindowEvent::ModifiersChanged(modifiers) => {
                        self.info.modifiers = modifiers.state();
                    },

                    WindowEvent::KeyboardInput {
                        event,
                        device_id,
                        is_synthetic,
                    } => {
                        if !is_synthetic {
                            // Handle tab navigation
                            if event.state == ElementState::Pressed {
                                use winit::keyboard::{KeyCode, PhysicalKey};
                                match event.physical_key {
                                    PhysicalKey::Code(KeyCode::Tab) => {
                                        // Check if modal overlays should block tab navigation... Handle tab navigation for focus
                                        if let Ok(mut manager) = self.info.focus_manager.lock() {
                                            if self.info.modifiers.shift_key() {
                                                // Shift+Tab: focus previous
                                                manager.focus_previous();
                                            } else {
                                                // Tab: focus next
                                                manager.focus_next();
                                            }
                                            self.update.insert(Update::FOCUS | Update::DRAW);
                                        }
                                        self.request_redraw();
                                        return; // Don't add tab keys to the key events list
                                    }
                                    PhysicalKey::Code(KeyCode::Escape) => {
                                        // Handle ESC key for modal overlays
                                    }
                                    _ => {}
                                }
                            }
                            
                            // Only add key events if not blocked by modal overlays
                            self.info.keys.push((device_id, event));
                            self.request_redraw();
                        }
                    },

                    WindowEvent::MouseInput {
                        device_id,
                        button,
                        state,
                    } => {
                        // Handle focus on mouse clicks
                        if button == MouseButton::Left && state == ElementState::Pressed {
                            if let Some(cursor_pos) = self.info.cursor_pos {
                                // Handle normal focus
                                if let Ok(mut manager) = self.info.focus_manager.lock() {
                                    if manager.handle_click(cursor_pos.x, cursor_pos.y) {
                                        self.update.insert(Update::FOCUS | Update::DRAW);
                                    }
                                }
                            }
                        }
                        
                        // Add mouse events
                        self.info.buttons.push((device_id, button, state));
                        self.request_redraw();
                    },

                    WindowEvent::MouseWheel { delta, .. } => {
                        self.info.mouse_scroll_delta = Some(delta);
                        self.request_redraw();
                    },

                    WindowEvent::Ime(ime_event) => {
                        self.info.ime_events.push(ime_event);
                        self.request_redraw();
                    },

                    WindowEvent::Destroyed => log::info!("Window destroyed! Exiting..."),

                    _ => (),
                }
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
