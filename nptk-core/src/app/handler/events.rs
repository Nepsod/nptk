use super::*;
use crate::app::context::AppContext;
use crate::app::info::AppKeyEvent;
use crate::app::update::Update;
use crate::menu::render::MenuGeometry;
use crate::shortcut::ShortcutRegistry;
use nalgebra::Vector2;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;

impl<W, S, F> AppHandler<W, S, F>
where
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    /// Handle a window event.
    pub(super) fn handle_window_event(&mut self, event: WindowEvent, event_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::Resized(new_size) => self.handle_resize(new_size, event_loop),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let physical = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .unwrap_or(winit::dpi::PhysicalSize::new(0, 0));

                if let Some(surface) = &mut self.surface {
                    if let Err(e) = surface.resize(physical.width, physical.height) {
                        log::error!("Failed to resize surface on scale change: {}", e);
                    }
                }

                let logical_size = physical.to_logical::<f64>(scale_factor);
                self.update_window_node_size(logical_size.width as u32, logical_size.height as u32);
                self.info.size = Vector2::new(logical_size.width, logical_size.height);
                self.request_redraw();
                self.update.insert(Update::DRAW | Update::LAYOUT);
            },
            WindowEvent::CloseRequested => self.handle_close_request(event_loop),
            WindowEvent::RedrawRequested => {
                log::trace!("RedrawRequested event received - rendering immediately");
                // Render immediately in response to RedrawRequested
                self.update.insert(Update::DRAW);
                self.update_internal(event_loop);
                
                // Request another redraw to keep the loop going
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            },
            WindowEvent::CursorLeft { .. } => {
                self.info.cursor_pos = None;
                self.request_redraw();
            },
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0);
                let logical = position.to_logical::<f64>(scale_factor);
                self.info.cursor_pos = Some(Vector2::new(logical.x, logical.y));
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
                self.handle_keyboard_input(event, device_id, is_synthetic);
            },
            WindowEvent::MouseInput {
                device_id,
                button,
                state,
            } => {
                self.handle_mouse_input(device_id, button, state);
            },
            WindowEvent::MouseWheel { delta, .. } => {
                self.info.mouse_scroll_delta = Some(delta);
                self.request_redraw();
            },
            WindowEvent::Ime(ime_event) => {
                self.info.ime_events.push(ime_event);
                self.request_redraw();
            },
            WindowEvent::Destroyed => {
                log::info!("Window destroyed! Cleaning up and exiting...");
                self.cleanup_resources();
            },
            _ => (),
        }
    }

    /// Handle window resize event.
    /// Optimized with throttling to avoid excessive layout recomputations during resize.
    pub(super) fn handle_resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        _event_loop: &ActiveEventLoop,
    ) {
        if new_size.width == 0 || new_size.height == 0 {
            log::debug!("Window size is 0x0, ignoring resize event.");
            return;
        }

        let scale_factor = self
            .window
            .as_ref()
            .map(|w| w.scale_factor())
            .unwrap_or(1.0);
        let logical_size = new_size.to_logical::<f64>(scale_factor);
        let new_logical_size = (logical_size.width as u32, logical_size.height as u32);

        // Update surface immediately
        if let Some(surface) = &mut self.surface {
            if let Err(e) = surface.resize(new_size.width, new_size.height) {
                log::error!("Failed to resize surface: {}", e);
            }
        }

        // Throttle layout updates: only schedule layout recomputation if:
        // 1. Size changed significantly (more than 2px to reduce jitter), OR
        // 2. Enough time has passed since last resize (8ms = ~120fps)
        const RESIZE_THROTTLE_MS: u64 = 8; // ~120fps
        const MIN_RESIZE_DELTA: i32 = 2; // Minimum pixel change to trigger layout update
        let now = std::time::Instant::now();
        let time_since_last_resize = now.duration_since(self.last_resize_time).as_millis() as u64;
        
        let width_delta = (new_logical_size.0 as i32 - self.last_window_size.0 as i32).abs();
        let height_delta = (new_logical_size.1 as i32 - self.last_window_size.1 as i32).abs();
        let size_changed_significantly = new_logical_size != self.last_window_size &&
            (width_delta >= MIN_RESIZE_DELTA || height_delta >= MIN_RESIZE_DELTA);

        if size_changed_significantly || time_since_last_resize >= RESIZE_THROTTLE_MS {
            // Update immediately and schedule layout recomputation
            self.update_window_node_size(new_logical_size.0, new_logical_size.1);
            self.info.size = Vector2::new(logical_size.width, logical_size.height);
            self.last_window_size = new_logical_size;
            self.last_resize_time = now;
            self.pending_resize = None;
            self.request_redraw();
            self.update.insert(Update::DRAW | Update::RESIZE);
        } else {
            // Store pending resize for later processing
            self.pending_resize = Some(new_logical_size);
            // Still request redraw for visual feedback
            self.request_redraw();
            self.update.insert(Update::DRAW);
        }
    }

    /// Handle window close request.
    pub(super) fn handle_close_request(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Window Close requested...");
        log::debug!("Cleaning up resources...");

        if self.config.window.close_on_request {
            // Perform cleanup before exiting
            self.cleanup_resources();
            event_loop.exit();
        }
    }

    /// Cleanup resources before application shutdown.
    fn cleanup_resources(&mut self) {
        log::debug!("Starting resource cleanup...");

        // Clear popup windows and their resources
        for (_, popup) in self.popup_windows.drain() {
            drop(popup);
        }

        #[cfg(all(target_os = "linux", feature = "wayland"))]
        {
            // Clear Wayland popups
            for (_, popup) in self.wayland_popups.drain() {
                drop(popup);
            }
        }

        // Clear GPU resources
        if let Some(renderer) = self.renderer.take() {
            drop(renderer);
        }
        
        if let Some(surface) = self.surface.take() {
            drop(surface);
        }

        if let Some(gpu_context) = self.gpu_context.take() {
            drop(gpu_context);
        }

        // Shutdown the task runner to prevent hanging
        crate::tasks::shutdown();

        log::debug!("Resource cleanup complete");
    }

    /// Handle keyboard input event.
    pub(super) fn handle_keyboard_input(
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
            
            // Try to dispatch shortcuts first (before special key handling)
            if let Some(update_flags) = self.shortcut_registry.try_dispatch(&event.physical_key, self.info.modifiers) {
                self.update.insert(update_flags);
                self.request_redraw();
                return;
            }
            
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Tab) => {
                    self.handle_tab_navigation();
                    self.request_redraw();
                    return;
                },
                PhysicalKey::Code(KeyCode::Escape) => {
                    if self.menu_manager.is_open() {
                        self.menu_manager.close();
                        self.update.insert(Update::DRAW);
                        return;
                    }
                },
                _ => {},
            }
        }

        let app_event = AppKeyEvent::from_winit(&event);
        self.info.keys.push((device_id, app_event));
        self.request_redraw();
    }

    /// Handle tab navigation for focus management.
    pub(super) fn handle_tab_navigation(&mut self) {
        let _ = self.info.batch_focus_operations(|manager| {
            if self.info.modifiers.shift_key() {
                manager.focus_previous();
            } else {
                manager.focus_next();
            }
            self.update.insert(Update::FOCUS | Update::DRAW);
        });
    }

    /// Handle mouse input event.
    pub(super) fn handle_mouse_input(
        &mut self,
        device_id: winit::event::DeviceId,
        button: MouseButton,
        state: ElementState,
    ) {
        if state == ElementState::Pressed {
            let Some(context) = self.context() else {
                // GPU context not available (e.g., during shutdown) - skip handling
                return;
            };
            if context.menu_manager.is_open() {
                if let Some(cursor_pos) = self.info.cursor_pos {
                    let cursor = vello::kurbo::Point::new(cursor_pos.x, cursor_pos.y);
                    let stack = context.menu_manager.get_stack();
                    
                    // Find which menu in the stack the cursor is over
                    for (template, position) in stack.iter().rev() {
                        let geometry = MenuGeometry::new(
                            template,
                            *position,
                            &mut self.text_render,
                            &mut self.info.font_context,
                        );
                        
                        if let Some(item_index) = geometry.hit_test_index(cursor) {
                            if let Some(item) = template.items.get(item_index) {
                                if item.enabled && !item.is_separator() {
                                    if item.has_submenu() {
                                        // Submenu already handled by hover
                                        self.update.insert(Update::DRAW);
                                        return;
                                    } else {
                                        // Execute action
                                        // Note: We need a way to route commands - for now use item.action
                                        if let Some(ref action) = item.action {
                                            action();
                                        }
                                        context.menu_manager.close();
                                        self.update.insert(Update::DRAW);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    
                    // Click outside any menu - close
                    context.menu_manager.close();
                    self.update.insert(Update::DRAW);
                    return;
                }
            }
        }

        if button == MouseButton::Left && state == ElementState::Pressed {
            if let Some(cursor_pos) = self.info.cursor_pos {
                let _ = self.info.batch_focus_operations(|manager| {
                    if manager.handle_click(cursor_pos.x, cursor_pos.y) {
                        self.update.insert(Update::FOCUS | Update::DRAW);
                    }
                });
            }
        }

        self.info.buttons.push((device_id, button, state));
        self.request_redraw();
    }

    pub(super) fn handle_update_flags(&mut self, event_loop: &ActiveEventLoop) {
        if self.update.get().intersects(Update::EVAL | Update::FORCE) {
            log::debug!("Evaluation update detected!");
            let platform = crate::platform::Platform::detect();
            if platform == crate::platform::Platform::Wayland {
                // Wayland without a winit window still relies on the event loop pumping updates.
            } else if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        if self.update.get().intersects(Update::EXIT) {
            self.cleanup_resources();
            event_loop.exit();
            return;
        }

        // Preserve LAYOUT flag if DRAW is also set (indicates resize detected during render)
        // This ensures layout recomputation happens in the next frame
        let flags_to_clear = if self.update.get().intersects(Update::DRAW | Update::LAYOUT) {
            // If both DRAW and LAYOUT are set, preserve both (resize detected)
            self.update.get() & !(Update::DRAW | Update::FORCE | Update::LAYOUT)
        } else {
            // Otherwise, clear all flags except DRAW and FORCE
            self.update.get() & !(Update::DRAW | Update::FORCE)
        };
        if flags_to_clear.bits() != 0 {
            self.update.set(self.update.get() & !flags_to_clear);
        }
    }
}
