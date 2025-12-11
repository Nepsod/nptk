use super::*;
use crate::app::update::Update;
use crate::layout::LayoutNode;
use crate::vgi::graphics_from_scene;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use vello::wgpu::{CommandEncoderDescriptor, TextureViewDescriptor};
use vello::RenderParams;
use winit::event_loop::{ActiveEventLoop, ControlFlow};

impl<T, W, S, F> AppHandler<T, W, S, F>
where
    T: Theme + Clone,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    pub(super) fn render_frame(&mut self, layout_node: &LayoutNode, event_loop: &ActiveEventLoop) {
        log::debug!("Draw update detected!");
        let render_start = Instant::now();

        self.scene.reset();
        let scene_reset_time = render_start.elapsed();

        let widget_render_time = self.render_widget(layout_node);
        let postfix_render_time = self.render_postfix(layout_node);

        self.render_context_menu();

        if let Some(render_times) = self.render_to_surface(
            render_start,
            scene_reset_time,
            widget_render_time,
            postfix_render_time,
            event_loop,
        ) {
            self.print_render_profile(render_times);
            self.update
                .set(self.update.get() & !(Update::DRAW | Update::FORCE));
        } else {
            let surface_size = match &self.surface {
                Some(crate::vgi::Surface::Winit(_)) => {
                    if let Some(window) = &self.window {
                        let size = window.inner_size();
                        (size.width, size.height)
                    } else {
                        (0, 0)
                    }
                },
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                Some(crate::vgi::Surface::Wayland(wayland_surface)) => wayland_surface.size(),
                None => (0, 0),
            };

            if surface_size.0 == 0 || surface_size.1 == 0 {
                log::debug!(
                    "render_frame() failed - surface size is 0x0, clearing DRAW flag to prevent infinite loop"
                );
                self.update.set(self.update.get() & !Update::DRAW);
            } else {
                log::debug!("render_frame() failed - keeping DRAW flag for retry, clearing FORCE");
                self.update.set(self.update.get() & !Update::FORCE);
            }
        }
    }

    fn render_widget(&mut self, layout_node: &LayoutNode) -> Duration {
        log::debug!("Rendering root widget...");
        let start = Instant::now();

        let context = self.context();
        let mut graphics =
            graphics_from_scene(&mut self.scene).expect("Failed to create graphics from scene");
        self.widget.as_mut().unwrap().render(
            graphics.as_mut(),
            &mut self.config.theme,
            layout_node,
            &mut self.info,
            context,
        );

        start.elapsed()
    }

    fn render_postfix(&mut self, layout_node: &LayoutNode) -> Duration {
        log::debug!("Rendering postfix content...");
        let start = Instant::now();
        let context = self.context();
        if let Some(mut graphics) = graphics_from_scene(&mut self.scene) {
            if let Some(widget) = &mut self.widget {
                widget.render_postfix(
                    &mut *graphics,
                    &mut self.config.theme,
                    layout_node,
                    &mut self.info,
                    context,
                );
            }
        }
        start.elapsed()
    }

    fn render_context_menu(&mut self) {
        let context = self.context();
        let stack = context.menu_manager.get_menu_stack();
        if stack.is_empty() {
            return;
        }
        if let Some(cursor_pos) = self.info.cursor_pos {
            let cursor = vello::kurbo::Point::new(cursor_pos.x, cursor_pos.y);

            let mut deepest_idx: Option<usize> = None;
            for (i, (menu, pos)) in stack.iter().enumerate() {
                let rect = crate::menu::get_menu_rect(
                    menu,
                    *pos,
                    &mut self.text_render,
                    &mut self.info.font_context,
                );
                if rect.contains(cursor) {
                    deepest_idx = Some(i);
                }
            }

            if let Some(idx) = deepest_idx {
                let (active_menu, active_pos) = stack[idx].clone();
                let mut new_stack = stack[..=idx].to_vec();

                if let Some((sub, sub_pos)) = crate::menu::hover_submenu(
                    &active_menu,
                    active_pos,
                    cursor,
                    &mut self.text_render,
                    &mut self.info.font_context,
                ) {
                    new_stack.push((sub, sub_pos));
                }

                context.menu_manager.set_stack(new_stack);
            }
        }

        if let Some(mut graphics) = graphics_from_scene(&mut self.scene) {
            let cursor_pos = self
                .info
                .cursor_pos
                .map(|p| vello::kurbo::Point::new(p.x, p.y));
            for (menu, position) in context.menu_manager.get_menu_stack() {
                crate::menu::render_context_menu(
                    graphics.as_mut(),
                    &menu,
                    position,
                    &mut self.config.theme,
                    &mut self.text_render,
                    &mut self.info.font_context,
                    cursor_pos,
                );
            }
        }
    }

    fn render_to_surface(
        &mut self,
        render_start: Instant,
        scene_reset_time: Duration,
        widget_render_time: Duration,
        postfix_render_time: Duration,
        event_loop: &ActiveEventLoop,
    ) -> Option<RenderTimes> {
        log::debug!("render_to_surface() called");

        if !self.async_init_complete.load(Ordering::Relaxed) {
            log::warn!("Async initialization not complete. Skipping render.");
            return None;
        }

        let renderer = self.renderer.as_mut()?;
        let gpu_context = self.gpu_context.as_ref()?;
        let devices = gpu_context.enumerate_devices();
        if devices.is_empty() {
            log::warn!("No devices found. Skipping render.");
            return None;
        }
        let device_handle = (self.config.render.device_selector)(devices);

        let surface = self.surface.as_mut()?;

        #[cfg(all(target_os = "linux", feature = "wayland"))]
        if let crate::vgi::Surface::Wayland(ref mut wayland_surface) = &mut *surface {
            if !wayland_surface.has_received_configure() {
                log::debug!("Wayland surface has not received configure yet. Skipping render.");
                return None;
            }
            if wayland_surface.requires_reconfigure() {
                let present_mode = match self.config.render.present_mode {
                    wgpu_types::PresentMode::AutoVsync => vello::wgpu::PresentMode::AutoVsync,
                    wgpu_types::PresentMode::AutoNoVsync => vello::wgpu::PresentMode::AutoNoVsync,
                    wgpu_types::PresentMode::Immediate => vello::wgpu::PresentMode::Immediate,
                    wgpu_types::PresentMode::Fifo => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::FifoRelaxed => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::Mailbox => vello::wgpu::PresentMode::Mailbox,
                };
                if let Err(e) = wayland_surface.configure_surface(
                    &device_handle.device,
                    wayland_surface.format(),
                    present_mode,
                ) {
                    log::warn!("Wayland reconfigure failed: {}", e);
                }
            }
            if !wayland_surface.is_configured() {
                log::warn!("Wayland surface not yet configured. Skipping render.");
                return None;
            }
        }

        let (width, height) = match &*surface {
            crate::vgi::Surface::Winit(_) => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    (size.width, size.height)
                } else {
                    log::warn!("Winit surface but no window available");
                    return None;
                }
            },
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            crate::vgi::Surface::Wayland(wayland_surface) => wayland_surface.size(),
        };

        if width == 0 || height == 0 {
            log::warn!("Surface invalid ({}x{}). Skipping render.", width, height);
            return None;
        }

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
                    return None;
                },
            }
        }

        #[cfg(all(target_os = "linux", feature = "wayland"))]
        if let crate::vgi::Surface::Wayland(ref mut wayland_surface) = &mut *surface {
            if wayland_surface.is_configured() && wayland_surface.requires_reconfigure() {
                let present_mode = match self.config.render.present_mode {
                    wgpu_types::PresentMode::AutoVsync => vello::wgpu::PresentMode::AutoVsync,
                    wgpu_types::PresentMode::AutoNoVsync => vello::wgpu::PresentMode::AutoNoVsync,
                    wgpu_types::PresentMode::Immediate => vello::wgpu::PresentMode::Immediate,
                    wgpu_types::PresentMode::Fifo => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::FifoRelaxed => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::Mailbox => vello::wgpu::PresentMode::Mailbox,
                };
                if let Err(e) = wayland_surface.configure_surface(
                    &device_handle.device,
                    wayland_surface.format(),
                    present_mode,
                ) {
                    log::warn!("Wayland proactive configure failed: {}", e);
                }
            }
        }

        let render_view = match surface.create_render_view(&device_handle.device, width, height) {
            Ok(view) => view,
            Err(err) => {
                log::error!("Failed to prepare render target: {}", err);
                return None;
            },
        };

        let surface_get_start = Instant::now();
        let surface_texture = match surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                log::warn!("Failed to get surface texture: {}, skipping render", e);
                return None;
            },
        };
        let surface_get_time = surface_get_start.elapsed();

        if let Some(window) = &self.window {
            window.pre_present_notify();
        }

        let gpu_render_start = Instant::now();
        if let Err(e) = renderer.render_to_view(
            &device_handle.device,
            &device_handle.queue,
            &self.scene,
            &render_view,
            &RenderParams {
                base_color: self.config.theme.window_background(),
                width,
                height,
                antialiasing_method: self.config.render.antialiasing,
            },
        ) {
            log::error!("Failed to render scene: {}", e);
            return None;
        }
        let gpu_render_time = gpu_render_start.elapsed();

        let mut encoder = device_handle
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main Surface Blit Encoder"),
            });

        let surface_view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        if let Err(e) = surface.blit_render_view(
            &device_handle.device,
            &mut encoder,
            &render_view,
            &surface_view,
        ) {
            log::error!("Failed to blit render view: {}", e);
            return None;
        }

        let present_start = Instant::now();
        device_handle
            .queue
            .submit(std::iter::once(encoder.finish()));

        match &mut *surface {
            crate::vgi::Surface::Winit(_) => {
                surface_texture.present();
            },
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            crate::vgi::Surface::Wayland(_) => {
                if let crate::vgi::Surface::Wayland(ref wayland_surface) = surface {
                    wayland_surface.prepare_frame();
                }
                surface_texture.present();
                if let Err(e) = surface.present() {
                    log::error!("Failed to present Wayland surface: {}", e);
                    return None;
                }
            },
        }

        event_loop.set_control_flow(ControlFlow::Poll);
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
}

pub(super) struct RenderTimes {
    pub(super) scene_reset_time: Duration,
    pub(super) widget_render_time: Duration,
    pub(super) postfix_render_time: Duration,
    pub(super) surface_get_time: Duration,
    pub(super) gpu_render_time: Duration,
    pub(super) present_time: Duration,
    pub(super) total_time: Duration,
}
