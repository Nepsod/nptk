#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::vgi::graphics_from_scene;
use crate::vgi::{DeviceHandle, GpuContext};
use crate::vgi::{Renderer, Surface, Scene, RendererOptions, SurfaceTrait};
use crate::layout::LayoutNode;
use taffy::prelude::*;
use crate::app::context::AppContext;
use nptk_services::settings::SettingsRegistry;
use nalgebra::Vector2;
use taffy::{
    AvailableSpace, Dimension, NodeId, PrintTree, Size, Style, TaffyResult, TaffyTree,
    TraversePartialTree,
};
use vello::wgpu::{CommandEncoderDescriptor, TextureFormat, TextureViewDescriptor};
use vello::{AaConfig, AaSupport, RenderParams};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::font_ctx::FontContext;
use crate::layout::StyleNode;
use crate::platform::Platform;
use taffy::style::Display;
#[cfg(target_os = "linux")]
use crate::app::info::WindowIdentity;
use crate::app::info::{AppInfo, AppKeyEvent};
use crate::app::update::{Update, UpdateManager};
use crate::config::MayConfig;
use crate::plugin::PluginManager;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::platform::wayland::events::{InputEvent, KeyboardEvent, PointerEvent};
use crate::widget::Widget;
use nptk_theme::theme::Theme;
#[cfg(target_os = "linux")]
use raw_window_handle::{RawWindowHandle, HasWindowHandle};
#[cfg(all(target_os = "linux", feature = "wayland"))]
use winit::event::DeviceId;
#[cfg(target_os = "linux")]
use winit::keyboard::{Key, KeyCode, ModifiersState, NativeKey, NativeKeyCode, PhysicalKey};

/// The core application handler. You should use [MayApp](crate::app::MayApp) instead for running applications.
pub struct AppHandler<T, W, S, F>
where
    T: Theme + Clone,
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
    gpu_context: Option<Arc<GpuContext>>,
    update: UpdateManager,
    last_update: Instant,
    plugins: PluginManager<T>,
    selected_device: usize,
    /// Tracks whether async initialization is complete
    async_init_complete: Arc<AtomicBool>,
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    wayland_pressed_keys: HashSet<u32>,
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    xkb_keymap: crate::app::keymap::XkbKeymapManager,
    text_render: crate::text_render::TextRenderContext,
    menu_manager: crate::menu::ContextMenuManager,
    popup_manager: crate::app::popup::PopupManager,
    popup_windows: std::collections::HashMap<WindowId, PopupWindow<T>>,
    /// Native Wayland popups (indexed by surface key u32)
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    wayland_popups: std::collections::HashMap<u32, PopupWindow<T>>,
    /// Counter for generating unique Wayland popup IDs
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    wayland_popup_id_counter: u32,
    settings: Arc<SettingsRegistry>,
}

struct PopupWindow<T: Theme + Clone> {
    /// Winit window (only for X11/Winit-based popups, None for native Wayland)
    window: Option<Arc<Window>>,
    renderer: Renderer,
    scene: Scene,
    surface: Surface,
    taffy: TaffyTree,
    root_node: NodeId,
    widget: Box<dyn Widget>,
    info: AppInfo,
    config: MayConfig<T>, // Each window needs its own config copy/ref for theme access
    /// Scale factor for HiDPI (1.0 for X11/Winit, 2.0 or higher for Wayland HiDPI)
    scale_factor: f32,
}
impl<T, W, S, F> AppHandler<T, W, S, F>
where
    T: Theme + Clone,
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
        let xkb_keymap = crate::app::keymap::XkbKeymapManager::new()
            .unwrap_or_else(|e| {
                log::warn!("Failed to create XKB keymap manager: {}", e);
                crate::app::keymap::XkbKeymapManager::default()
            });

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
            menu_manager: crate::menu::ContextMenuManager::new(),
            popup_manager: crate::app::popup::PopupManager::new(),
            popup_windows: std::collections::HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_popups: std::collections::HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            wayland_popup_id_counter: 0,
            settings,
        }
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn process_wayland_input_events(&mut self) {
        use crate::platform::Platform;
        use wayland_client::protocol::{wl_keyboard, wl_pointer};
        use winit::event::ElementState;
        use winit::event::MouseButton;

        if Platform::detect() != Platform::Wayland {
            return;
        }

        let events = {
            let Some(surface) = self.surface.as_mut() else {
                return;
            };
            surface.take_wayland_input_events()
        };
        log::debug!("process_wayland_input_events: got {} events", events.len());
        if events.is_empty() {
            return;
        }

        let pointer_device = DeviceId::dummy();
        let mut pending_scroll: Option<(f64, f64)> = None;
        let mut scroll_is_line = false;
        let mut axis_source: Option<wl_pointer::AxisSource> = None;

        let map_pointer_button = |button: u32| -> MouseButton {
            match button {
                0x110 => MouseButton::Left,
                0x111 => MouseButton::Right,
                0x112 => MouseButton::Middle,
                other => MouseButton::Other((other & 0xFFFF) as u16),
            }
        };

        for event in events {
            match event {
                InputEvent::Pointer(pointer_event) => match pointer_event {
                    PointerEvent::Enter {
                        surface_x,
                        surface_y,
                        ..
                    } => {
                        self.info.cursor_pos = Some(Vector2::new(surface_x, surface_y));
                        self.request_redraw();
                    },
                    PointerEvent::Leave { .. } => {
                        self.info.cursor_pos = None;
                        self.request_redraw();
                    },
                    PointerEvent::Motion {
                        surface_x,
                        surface_y,
                        ..
                    } => {
                        self.info.cursor_pos = Some(Vector2::new(surface_x, surface_y));
                        self.request_redraw();
                    },
                    PointerEvent::Button { button, state, .. } => {
                        let element_state = match state {
                            wl_pointer::ButtonState::Pressed => ElementState::Pressed,
                            wl_pointer::ButtonState::Released => ElementState::Released,
                            _ => continue,
                        };
                        let mapped_button = map_pointer_button(button);
                        self.handle_mouse_input(pointer_device, mapped_button, element_state);
                    },
                    PointerEvent::Axis {
                        horizontal,
                        vertical,
                    } => {
                        let (mut h, mut v) = pending_scroll.unwrap_or((0.0, 0.0));
                        if let Some(value) = horizontal {
                            h += value;
                        }
                        if let Some(value) = vertical {
                            v += value;
                        }
                        pending_scroll = Some((h, v));
                    },
                    PointerEvent::AxisSource { source } => {
                        axis_source = Some(source);
                    },
                    PointerEvent::AxisStop => {
                        self.flush_wayland_scroll(
                            &mut pending_scroll,
                            &mut scroll_is_line,
                            &mut axis_source,
                        );
                    },
                    PointerEvent::AxisDiscrete { axis, discrete } => {
                        let (mut h, mut v) = pending_scroll.unwrap_or((0.0, 0.0));
                        match axis {
                            wl_pointer::Axis::HorizontalScroll => h += discrete as f64,
                            wl_pointer::Axis::VerticalScroll => v += discrete as f64,
                            _ => {},
                        }
                        pending_scroll = Some((h, v));
                        scroll_is_line = true;
                    },
                    PointerEvent::AxisValue120 { axis, value120 } => {
                        let (mut h, mut v) = pending_scroll.unwrap_or((0.0, 0.0));
                        let value = (value120 as f64) / 120.0;
                        match axis {
                            wl_pointer::Axis::HorizontalScroll => h += value,
                            wl_pointer::Axis::VerticalScroll => v += value,
                            _ => {},
                        }
                        pending_scroll = Some((h, v));
                        scroll_is_line = true;
                    },
                    PointerEvent::Frame => {
                        self.flush_wayland_scroll(
                            &mut pending_scroll,
                            &mut scroll_is_line,
                            &mut axis_source,
                        );
                    },
                },
                InputEvent::Keyboard(key_event) => {
                    match key_event {
                        KeyboardEvent::Enter => {
                            self.wayland_pressed_keys.clear();
                            self.info.modifiers = ModifiersState::empty();
                        },
                        KeyboardEvent::Leave => {
                            self.wayland_pressed_keys.clear();
                            self.info.modifiers = ModifiersState::empty();
                        },
                        KeyboardEvent::Key { keycode, state } => {
                            let evdev = Self::normalize_wayland_keycode(keycode);
                            let element_state = match state {
                                wl_keyboard::KeyState::Pressed => ElementState::Pressed,
                                wl_keyboard::KeyState::Released => ElementState::Released,
                                _ => continue,
                            };

                            let repeat = match element_state {
                                ElementState::Pressed => !self.wayland_pressed_keys.insert(evdev),
                                ElementState::Released => {
                                    self.wayland_pressed_keys.remove(&evdev);
                                    false
                                },
                            };

                            self.update_wayland_modifiers_state();

                            // Use XKB for keycode decoding if available, otherwise fall back to hardcoded mapping
                                let (physical_key, text) = if self.xkb_keymap.is_ready() {
                                    // Wayland keycodes are evdev scancodes + 8, which matches XKB keycodes
                                    use xkbcommon_dl::xkb_key_direction;
                                    let direction = match element_state {
                                        ElementState::Pressed => xkb_key_direction::XKB_KEY_DOWN,
                                        ElementState::Released => xkb_key_direction::XKB_KEY_UP,
                                    };
                                    // If we assume keycode is raw evdev (because we removed the -8 in normalize),
                                    // then we must ADD 8 for XKB lookup, as XKB expects evdev+8.
                                    let xkb_keycode = keycode + 8;
                                    let keysym = self.xkb_keymap.keycode_to_keysym(xkb_keycode, direction);
                                    log::debug!("XKB keycode {} (direction={:?}) -> keysym {:?}", xkb_keycode, direction, keysym);
                                    let utf8_text = if element_state == ElementState::Pressed {
                                        use xkbcommon_dl::xkb_key_direction;
                                        self.xkb_keymap.keycode_to_utf8(xkb_keycode, xkb_key_direction::XKB_KEY_DOWN)
                                    } else {
                                        None
                                    };
                                
                                let physical = if let Some(ks) = keysym {
                                    // Special case: keycode 68 with keysym 0xFFBF (65471) is likely F10
                                    // Some keyboards/XKB return wrong keysym for F10
                                    if keycode == 68 && (ks == 0xFFBF || ks == 65471) {
                                        log::debug!("Detected F10: keycode=68, keysym=0xFFBF, using F10 directly");
                                        PhysicalKey::Code(KeyCode::F10)
                                    } else {
                                        let mapped = Self::keysym_to_physical_key(ks);
                                        log::debug!("XKB keysym {} (0x{:X}) -> physical_key {:?}", ks, ks, mapped);
                                        // If keysym doesn't match, fall back to hardcoded mapping
                                        match &mapped {
                                            PhysicalKey::Unidentified(_) => {
                                                log::debug!("Keysym {} (0x{:X}) not recognized, falling back to hardcoded mapping (keycode={}, evdev={})", ks, ks, keycode, Self::normalize_wayland_keycode(keycode));
                                                let evdev = Self::normalize_wayland_keycode(keycode);
                                                let fallback = Self::map_wayland_physical_key(evdev, keycode);
                                                log::debug!("Hardcoded mapping: evdev {} -> {:?}", evdev, fallback);
                                                fallback
                                            },
                                            _ => mapped,
                                        }
                                    }
                                } else {
                                    // Fallback to hardcoded mapping if xkb fails
                                    log::debug!("XKB keysym lookup failed, falling back to hardcoded mapping");
                                    let evdev = Self::normalize_wayland_keycode(keycode);
                                    Self::map_wayland_physical_key(evdev, keycode)
                                };
                                
                                (physical, utf8_text)
                            } else {
                                // Fallback to hardcoded mapping if xkb is not ready
                                log::debug!("XKB keymap not ready, using hardcoded mapping for keycode {}", keycode);
                                let evdev = Self::normalize_wayland_keycode(keycode);
                                let physical = Self::map_wayland_physical_key(evdev, keycode);
                                let text = if element_state == ElementState::Pressed {
                                    Self::map_wayland_text(evdev, self.info.modifiers.shift_key())
                                } else {
                                    None
                                };
                                (physical, text)
                            };
                            
                                let logical_key = text
                                    .as_ref()
                                    .map(|value| Key::Character(value.clone().into()))
                                    .unwrap_or_else(|| {
                                        // Map physical keys to logical keys for non-text keys
                                        use winit::keyboard::{Key, NamedKey};
                                        match physical_key {
                                            PhysicalKey::Code(KeyCode::Escape) => Key::Named(NamedKey::Escape),
                                            PhysicalKey::Code(KeyCode::Tab) => Key::Named(NamedKey::Tab),
                                            PhysicalKey::Code(KeyCode::Backspace) => Key::Named(NamedKey::Backspace),
                                            PhysicalKey::Code(KeyCode::Enter) => Key::Named(NamedKey::Enter),
                                            PhysicalKey::Code(KeyCode::Space) => Key::Named(NamedKey::Space),
                                            PhysicalKey::Code(KeyCode::Delete) => Key::Named(NamedKey::Delete),
                                            PhysicalKey::Code(KeyCode::Insert) => Key::Named(NamedKey::Insert),
                                            PhysicalKey::Code(KeyCode::Home) => Key::Named(NamedKey::Home),
                                            PhysicalKey::Code(KeyCode::End) => Key::Named(NamedKey::End),
                                            PhysicalKey::Code(KeyCode::PageUp) => Key::Named(NamedKey::PageUp),
                                            PhysicalKey::Code(KeyCode::PageDown) => Key::Named(NamedKey::PageDown),
                                            PhysicalKey::Code(KeyCode::ArrowUp) => Key::Named(NamedKey::ArrowUp),
                                            PhysicalKey::Code(KeyCode::ArrowDown) => Key::Named(NamedKey::ArrowDown),
                                            PhysicalKey::Code(KeyCode::ArrowLeft) => Key::Named(NamedKey::ArrowLeft),
                                            PhysicalKey::Code(KeyCode::ArrowRight) => Key::Named(NamedKey::ArrowRight),
                                            PhysicalKey::Code(KeyCode::F1) => Key::Named(NamedKey::F1),
                                            PhysicalKey::Code(KeyCode::F2) => Key::Named(NamedKey::F2),
                                            PhysicalKey::Code(KeyCode::F3) => Key::Named(NamedKey::F3),
                                            PhysicalKey::Code(KeyCode::F4) => Key::Named(NamedKey::F4),
                                            PhysicalKey::Code(KeyCode::F5) => Key::Named(NamedKey::F5),
                                            PhysicalKey::Code(KeyCode::F6) => Key::Named(NamedKey::F6),
                                            PhysicalKey::Code(KeyCode::F7) => Key::Named(NamedKey::F7),
                                            PhysicalKey::Code(KeyCode::F8) => Key::Named(NamedKey::F8),
                                            PhysicalKey::Code(KeyCode::F9) => Key::Named(NamedKey::F9),
                                            PhysicalKey::Code(KeyCode::F10) => Key::Named(NamedKey::F10),
                                            PhysicalKey::Code(KeyCode::F11) => Key::Named(NamedKey::F11),
                                            PhysicalKey::Code(KeyCode::F12) => Key::Named(NamedKey::F12),
                                            PhysicalKey::Code(KeyCode::ShiftLeft) | PhysicalKey::Code(KeyCode::ShiftRight) => Key::Named(NamedKey::Shift),
                                            PhysicalKey::Code(KeyCode::ControlLeft) | PhysicalKey::Code(KeyCode::ControlRight) => Key::Named(NamedKey::Control),
                                            PhysicalKey::Code(KeyCode::AltLeft) | PhysicalKey::Code(KeyCode::AltRight) => Key::Named(NamedKey::Alt),
                                            PhysicalKey::Code(KeyCode::SuperLeft) | PhysicalKey::Code(KeyCode::SuperRight) => Key::Named(NamedKey::Super),
                                            _ => Key::Unidentified(NativeKey::Unidentified),
                                        }
                                    });

                            let app_event = AppKeyEvent {
                                physical_key,
                                logical_key,
                                text,
                                state: element_state,
                                repeat,
                            };

                            log::debug!("Wayland keyboard event: keycode={}, evdev={}, physical_key={:?}, state={:?}", keycode, evdev, physical_key, element_state);

                            let keyboard_device = DeviceId::dummy();
                            self.info.keys.push((keyboard_device, app_event));
                            self.request_redraw();
                        },
                        KeyboardEvent::Modifiers { mods_depressed, mods_latched, mods_locked, group } => {
                            // Update XKB state with modifier changes
                            self.xkb_keymap.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
                            // Also update via key state tracking for compatibility
                            self.update_wayland_modifiers_state();
                        },
                        KeyboardEvent::RepeatInfo { .. } => {
                            // Unsupported repeat customization.
                        },
                        KeyboardEvent::Keymap { keymap_string } => {
                            log::info!("Received keymap ({} bytes), updating XKB keymap manager", keymap_string.len());
                            if let Err(e) = self.xkb_keymap.update_keymap(&keymap_string) {
                                log::warn!("Failed to update XKB keymap: {}", e);
                            } else {
                                log::info!("XKB keymap updated successfully, ready={}", self.xkb_keymap.is_ready());
                            }
                        },
                    }
                },
                InputEvent::Touch(_touch_event) => {
                    // Touch input handling - TODO: implement touch event processing
                    // For now, touch events are ignored
                },
                InputEvent::Tablet(_tablet_event) => {
                    // Tablet input handling - TODO: implement tablet event processing
                    // For now, tablet events are ignored
                },
            }
        }

        self.flush_wayland_scroll(&mut pending_scroll, &mut scroll_is_line, &mut axis_source);
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn flush_wayland_scroll(
        &mut self,
        pending: &mut Option<(f64, f64)>,
        scroll_is_line: &mut bool,
        axis_source: &mut Option<wayland_client::protocol::wl_pointer::AxisSource>,
    ) {
        use wayland_client::protocol::wl_pointer::AxisSource;
        use winit::dpi::PhysicalPosition;
        use winit::event::MouseScrollDelta;

        if let Some((horizontal, vertical)) = pending.take() {
            if horizontal != 0.0 || vertical != 0.0 {
                let delta = match axis_source {
                    Some(AxisSource::Finger) => {
                        MouseScrollDelta::PixelDelta(PhysicalPosition::new(horizontal, vertical))
                    },
                    Some(AxisSource::Wheel)
                    | Some(AxisSource::WheelTilt)
                    | Some(AxisSource::Continuous)
                    | None => {
                        if *scroll_is_line {
                            MouseScrollDelta::LineDelta(horizontal as f32, vertical as f32)
                        } else {
                            MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                                horizontal, vertical,
                            ))
                        }
                    },
                    _ => MouseScrollDelta::PixelDelta(PhysicalPosition::new(horizontal, vertical)),
                };
                self.info.mouse_scroll_delta = Some(delta);
                self.request_redraw();
            }
        }
        *scroll_is_line = false;
        *axis_source = None;
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn normalize_wayland_keycode(keycode: u32) -> u32 {
        // The user's compositor seems to be sending raw evdev codes (e.g. 29 for Ctrl)
        // instead of XKB codes (e.g. 37 for Ctrl).
        // Standard behavior is keycode - 8, but here we need identity.
        // However, to be safe(r), let's try to detect if we need to offset.
        // But for now, let's assume raw evdev based on the 'Ctrl=y' (29->21) symptom.
        keycode
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn keysym_to_physical_key(keysym: u32) -> PhysicalKey {
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        use xkbcommon_dl::keysyms;
        
        // Map XKB keysyms to winit KeyCodes
        match keysym {
            keysyms::F1 => PhysicalKey::Code(KeyCode::F1),
            keysyms::F2 => PhysicalKey::Code(KeyCode::F2),
            keysyms::F3 => PhysicalKey::Code(KeyCode::F3),
            keysyms::F4 => PhysicalKey::Code(KeyCode::F4),
            keysyms::F5 => PhysicalKey::Code(KeyCode::F5),
            keysyms::F6 => PhysicalKey::Code(KeyCode::F6),
            keysyms::F7 => PhysicalKey::Code(KeyCode::F7),
            keysyms::F8 => PhysicalKey::Code(KeyCode::F8),
            keysyms::F9 => PhysicalKey::Code(KeyCode::F9),
            keysyms::F10 => PhysicalKey::Code(KeyCode::F10),
            keysyms::F11 => PhysicalKey::Code(KeyCode::F11),
            keysyms::F12 => PhysicalKey::Code(KeyCode::F12),
            keysyms::F13 => PhysicalKey::Code(KeyCode::F13),
            keysyms::F14 => PhysicalKey::Code(KeyCode::F14),
            keysyms::F15 => PhysicalKey::Code(KeyCode::F15),
            keysyms::F16 => PhysicalKey::Code(KeyCode::F16),
            keysyms::F17 => PhysicalKey::Code(KeyCode::F17),
            keysyms::F18 => PhysicalKey::Code(KeyCode::F18),
            keysyms::F19 => PhysicalKey::Code(KeyCode::F19),
            keysyms::F20 => PhysicalKey::Code(KeyCode::F20),
            keysyms::F21 => PhysicalKey::Code(KeyCode::F21),
            keysyms::F22 => PhysicalKey::Code(KeyCode::F22),
            keysyms::F23 => PhysicalKey::Code(KeyCode::F23),
            keysyms::F24 => PhysicalKey::Code(KeyCode::F24),
            keysyms::Escape => PhysicalKey::Code(KeyCode::Escape),
            keysyms::Tab => PhysicalKey::Code(KeyCode::Tab),
            keysyms::BackSpace => PhysicalKey::Code(KeyCode::Backspace),
            keysyms::Return => PhysicalKey::Code(KeyCode::Enter),
            keysyms::space => PhysicalKey::Code(KeyCode::Space),
            keysyms::Home => PhysicalKey::Code(KeyCode::Home),
            keysyms::End => PhysicalKey::Code(KeyCode::End),
            keysyms::Up => PhysicalKey::Code(KeyCode::ArrowUp),
            keysyms::Down => PhysicalKey::Code(KeyCode::ArrowDown),
            keysyms::Left => PhysicalKey::Code(KeyCode::ArrowLeft),
            keysyms::Right => PhysicalKey::Code(KeyCode::ArrowRight),
            keysyms::Page_Up => PhysicalKey::Code(KeyCode::PageUp),
            keysyms::Page_Down => PhysicalKey::Code(KeyCode::PageDown),
            keysyms::Insert => PhysicalKey::Code(KeyCode::Insert),
            keysyms::Delete => PhysicalKey::Code(KeyCode::Delete),
            keysyms::Shift_L => PhysicalKey::Code(KeyCode::ShiftLeft),
            keysyms::Shift_R => PhysicalKey::Code(KeyCode::ShiftRight),
            keysyms::Control_L => PhysicalKey::Code(KeyCode::ControlLeft),
            keysyms::Control_R => PhysicalKey::Code(KeyCode::ControlRight),
            keysyms::Alt_L => PhysicalKey::Code(KeyCode::AltLeft),
            keysyms::Alt_R => PhysicalKey::Code(KeyCode::AltRight),
            keysyms::Super_L => PhysicalKey::Code(KeyCode::SuperLeft),
            keysyms::Super_R => PhysicalKey::Code(KeyCode::SuperRight),
            // Number keys
            keysyms::_1 => PhysicalKey::Code(KeyCode::Digit1),
            keysyms::_2 => PhysicalKey::Code(KeyCode::Digit2),
            keysyms::_3 => PhysicalKey::Code(KeyCode::Digit3),
            keysyms::_4 => PhysicalKey::Code(KeyCode::Digit4),
            keysyms::_5 => PhysicalKey::Code(KeyCode::Digit5),
            keysyms::_6 => PhysicalKey::Code(KeyCode::Digit6),
            keysyms::_7 => PhysicalKey::Code(KeyCode::Digit7),
            keysyms::_8 => PhysicalKey::Code(KeyCode::Digit8),
            keysyms::_9 => PhysicalKey::Code(KeyCode::Digit9),
            keysyms::_0 => PhysicalKey::Code(KeyCode::Digit0),
            // Letter keys
            keysyms::a => PhysicalKey::Code(KeyCode::KeyA),
            keysyms::b => PhysicalKey::Code(KeyCode::KeyB),
            keysyms::c => PhysicalKey::Code(KeyCode::KeyC),
            keysyms::d => PhysicalKey::Code(KeyCode::KeyD),
            keysyms::e => PhysicalKey::Code(KeyCode::KeyE),
            keysyms::f => PhysicalKey::Code(KeyCode::KeyF),
            keysyms::g => PhysicalKey::Code(KeyCode::KeyG),
            keysyms::h => PhysicalKey::Code(KeyCode::KeyH),
            keysyms::i => PhysicalKey::Code(KeyCode::KeyI),
            keysyms::j => PhysicalKey::Code(KeyCode::KeyJ),
            keysyms::k => PhysicalKey::Code(KeyCode::KeyK),
            keysyms::l => PhysicalKey::Code(KeyCode::KeyL),
            keysyms::m => PhysicalKey::Code(KeyCode::KeyM),
            keysyms::n => PhysicalKey::Code(KeyCode::KeyN),
            keysyms::o => PhysicalKey::Code(KeyCode::KeyO),
            keysyms::p => PhysicalKey::Code(KeyCode::KeyP),
            keysyms::q => PhysicalKey::Code(KeyCode::KeyQ),
            keysyms::r => PhysicalKey::Code(KeyCode::KeyR),
            keysyms::s => PhysicalKey::Code(KeyCode::KeyS),
            keysyms::t => PhysicalKey::Code(KeyCode::KeyT),
            keysyms::u => PhysicalKey::Code(KeyCode::KeyU),
            keysyms::v => PhysicalKey::Code(KeyCode::KeyV),
            keysyms::w => PhysicalKey::Code(KeyCode::KeyW),
            keysyms::x => PhysicalKey::Code(KeyCode::KeyX),
            keysyms::y => PhysicalKey::Code(KeyCode::KeyY),
            keysyms::z => PhysicalKey::Code(KeyCode::KeyZ),
            _ => PhysicalKey::Unidentified(NativeKeyCode::Xkb(keysym)),
        }
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn map_wayland_physical_key(evdev: u32, raw: u32) -> PhysicalKey {
        match evdev {
            1 => PhysicalKey::Code(KeyCode::Escape),
            2 => PhysicalKey::Code(KeyCode::Digit1),
            3 => PhysicalKey::Code(KeyCode::Digit2),
            4 => PhysicalKey::Code(KeyCode::Digit3),
            5 => PhysicalKey::Code(KeyCode::Digit4),
            6 => PhysicalKey::Code(KeyCode::Digit5),
            7 => PhysicalKey::Code(KeyCode::Digit6),
            8 => PhysicalKey::Code(KeyCode::Digit7),
            9 => PhysicalKey::Code(KeyCode::Digit8),
            10 => PhysicalKey::Code(KeyCode::Digit9),
            11 => PhysicalKey::Code(KeyCode::Digit0),
            12 => PhysicalKey::Code(KeyCode::Minus),
            13 => PhysicalKey::Code(KeyCode::Equal),
            14 => PhysicalKey::Code(KeyCode::Backspace),
            15 => PhysicalKey::Code(KeyCode::Tab),
            16 => PhysicalKey::Code(KeyCode::KeyQ),
            17 => PhysicalKey::Code(KeyCode::KeyW),
            18 => PhysicalKey::Code(KeyCode::KeyE),
            19 => PhysicalKey::Code(KeyCode::KeyR),
            20 => PhysicalKey::Code(KeyCode::KeyT),
            21 => PhysicalKey::Code(KeyCode::KeyY),
            22 => PhysicalKey::Code(KeyCode::KeyU),
            23 => PhysicalKey::Code(KeyCode::KeyI),
            24 => PhysicalKey::Code(KeyCode::KeyO),
            25 => PhysicalKey::Code(KeyCode::KeyP),
            26 => PhysicalKey::Code(KeyCode::BracketLeft),
            27 => PhysicalKey::Code(KeyCode::BracketRight),
            28 => PhysicalKey::Code(KeyCode::Enter),
            29 => PhysicalKey::Code(KeyCode::ControlLeft),
            30 => PhysicalKey::Code(KeyCode::KeyA),
            31 => PhysicalKey::Code(KeyCode::KeyS),
            32 => PhysicalKey::Code(KeyCode::KeyD),
            33 => PhysicalKey::Code(KeyCode::KeyF),
            34 => PhysicalKey::Code(KeyCode::KeyG),
            35 => PhysicalKey::Code(KeyCode::KeyH),
            36 => PhysicalKey::Code(KeyCode::KeyJ),
            37 => PhysicalKey::Code(KeyCode::KeyK),
            38 => PhysicalKey::Code(KeyCode::KeyL),
            39 => PhysicalKey::Code(KeyCode::Semicolon),
            40 => PhysicalKey::Code(KeyCode::Quote),
            41 => PhysicalKey::Code(KeyCode::Backquote),
            42 => PhysicalKey::Code(KeyCode::ShiftLeft),
            43 => PhysicalKey::Code(KeyCode::Backslash),
            44 => PhysicalKey::Code(KeyCode::KeyZ),
            45 => PhysicalKey::Code(KeyCode::KeyX),
            46 => PhysicalKey::Code(KeyCode::KeyC),
            47 => PhysicalKey::Code(KeyCode::KeyV),
            48 => PhysicalKey::Code(KeyCode::KeyB),
            49 => PhysicalKey::Code(KeyCode::KeyN),
            50 => PhysicalKey::Code(KeyCode::KeyM),
            51 => PhysicalKey::Code(KeyCode::Comma),
            52 => PhysicalKey::Code(KeyCode::Period),
            53 => PhysicalKey::Code(KeyCode::Slash),
            54 => PhysicalKey::Code(KeyCode::ShiftRight),
            56 => PhysicalKey::Code(KeyCode::AltLeft),
            57 => PhysicalKey::Code(KeyCode::Space),
            58 => PhysicalKey::Code(KeyCode::CapsLock),
            59 => PhysicalKey::Code(KeyCode::F1),
            60 => PhysicalKey::Code(KeyCode::F2),
            61 => PhysicalKey::Code(KeyCode::F3),
            62 => PhysicalKey::Code(KeyCode::F4),
            63 => PhysicalKey::Code(KeyCode::F5),
            64 => PhysicalKey::Code(KeyCode::F6),
            65 => PhysicalKey::Code(KeyCode::F7),
            66 => PhysicalKey::Code(KeyCode::F8),
            67 => PhysicalKey::Code(KeyCode::F9),
            68 => PhysicalKey::Code(KeyCode::F10),
            69 => PhysicalKey::Code(KeyCode::F11),
            70 => PhysicalKey::Code(KeyCode::F12),
            71 => PhysicalKey::Code(KeyCode::F13),
            72 => PhysicalKey::Code(KeyCode::F14),
            73 => PhysicalKey::Code(KeyCode::F15),
            74 => PhysicalKey::Code(KeyCode::F16),
            75 => PhysicalKey::Code(KeyCode::F17),
            76 => PhysicalKey::Code(KeyCode::F18),
            79 => PhysicalKey::Code(KeyCode::Numpad7),
            80 => PhysicalKey::Code(KeyCode::Numpad8),
            81 => PhysicalKey::Code(KeyCode::Numpad9),
            83 => PhysicalKey::Code(KeyCode::Numpad4),
            84 => PhysicalKey::Code(KeyCode::Numpad5),
            85 => PhysicalKey::Code(KeyCode::Numpad6),
            86 => PhysicalKey::Code(KeyCode::NumpadAdd),
            87 => PhysicalKey::Code(KeyCode::Numpad1),
            88 => PhysicalKey::Code(KeyCode::Numpad2),
            89 => PhysicalKey::Code(KeyCode::Numpad3),
            90 => PhysicalKey::Code(KeyCode::Numpad0),
            91 => PhysicalKey::Code(KeyCode::NumpadDecimal),
            102 => PhysicalKey::Code(KeyCode::Home),
            103 => PhysicalKey::Code(KeyCode::ArrowUp),
            104 => PhysicalKey::Code(KeyCode::PageUp),
            105 => PhysicalKey::Code(KeyCode::ArrowLeft),
            106 => PhysicalKey::Code(KeyCode::ArrowRight),
            107 => PhysicalKey::Code(KeyCode::End),
            108 => PhysicalKey::Code(KeyCode::ArrowDown),
            109 => PhysicalKey::Code(KeyCode::PageDown),
            110 => PhysicalKey::Code(KeyCode::Insert),
            111 => PhysicalKey::Code(KeyCode::Delete),
            125 => PhysicalKey::Code(KeyCode::SuperLeft),
            126 => PhysicalKey::Code(KeyCode::SuperRight),
            127 => PhysicalKey::Code(KeyCode::ContextMenu),
            _ => PhysicalKey::Unidentified(NativeKeyCode::Xkb(raw)),
        }
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn map_wayland_text(evdev: u32, shift: bool) -> Option<String> {
        let ch = match evdev {
            2 => Some('1'),
            3 => Some('2'),
            4 => Some('3'),
            5 => Some('4'),
            6 => Some('5'),
            7 => Some('6'),
            8 => Some('7'),
            9 => Some('8'),
            10 => Some('9'),
            11 => Some('0'),
            16 => Some('q'),
            17 => Some('w'),
            18 => Some('e'),
            19 => Some('r'),
            20 => Some('t'),
            21 => Some('y'),
            22 => Some('u'),
            23 => Some('i'),
            24 => Some('o'),
            25 => Some('p'),
            30 => Some('a'),
            31 => Some('s'),
            32 => Some('d'),
            33 => Some('f'),
            34 => Some('g'),
            35 => Some('h'),
            36 => Some('j'),
            37 => Some('k'),
            38 => Some('l'),
            44 => Some('z'),
            45 => Some('x'),
            46 => Some('c'),
            47 => Some('v'),
            48 => Some('b'),
            49 => Some('n'),
            50 => Some('m'),
            57 => Some(' '),
            _ => None,
        }?;

        let rendered = if shift { ch.to_ascii_uppercase() } else { ch };
        Some(rendered.to_string())
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn update_wayland_modifiers_state(&mut self) {
        const LEFT_SHIFT: u32 = 42;
        const RIGHT_SHIFT: u32 = 54;
        const LEFT_CTRL: u32 = 29;
        const RIGHT_CTRL: u32 = 97;
        const LEFT_ALT: u32 = 56;
        const RIGHT_ALT: u32 = 100;
        const LEFT_SUPER: u32 = 125;
        const RIGHT_SUPER: u32 = 126;

        let mut mods = ModifiersState::empty();
        let pressed = &self.wayland_pressed_keys;

        if pressed.contains(&LEFT_SHIFT) || pressed.contains(&RIGHT_SHIFT) {
            mods.set(ModifiersState::SHIFT, true);
        }
        if pressed.contains(&LEFT_CTRL) || pressed.contains(&RIGHT_CTRL) {
            mods.set(ModifiersState::CONTROL, true);
        }
        if pressed.contains(&LEFT_ALT) || pressed.contains(&RIGHT_ALT) {
            mods.set(ModifiersState::ALT, true);
        }
        if pressed.contains(&LEFT_SUPER) || pressed.contains(&RIGHT_SUPER) {
            mods.set(ModifiersState::SUPER, true);
        }

        self.info.modifiers = mods;
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
    fn collect_layout_impl(&mut self, node: NodeId, style: &StyleNode, parent_x: f32, parent_y: f32) -> TaffyResult<LayoutNode> {
        let mut children = Vec::new();
        let taffy_child_count = self.taffy.child_count(node);
        let style_child_count = style.children.len();

        // Count visible children in style
        let visible_style_children: Vec<_> = style.children.iter()
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
                children.push(self.collect_layout_impl(child_node, child_style, child_content_x, child_content_y)?);
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
                format!("first_child_pos=({:.1}, {:.1})", children[0].layout.location.x, children[0].layout.location.y)
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
        self.update_plugins(event_loop);

        let mut layout_node = self.ensure_layout_initialized();
        layout_node = self.update_layout_if_needed(layout_node);

        // Update widget - this may set LAYOUT or FORCE flags if visibility changes
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
            self.render_frame(&layout_node, event_loop);
        }

        self.handle_update_flags(event_loop);
        self.info.reset();
        self.update_diagnostics();
    }

    /// Update plugins with current state.
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
            log::info!("Root widget has no children in layout tree after rebuild, returning empty layout");
            return LayoutNode {
                layout: *self.taffy.get_final_layout(self.window_node),
                children: vec![],
            };
        }
        let new_layout = self.collect_layout(
            self.taffy.child_at_index(self.window_node, 0).unwrap(),
            &style,
        )
        .expect("Failed to collect layout");
        
        // Log the difference in positions to verify layout is updating
        if !new_layout.children.is_empty() && !layout_node.children.is_empty() {
            let old_pos = layout_node.children[0].layout.location.y;
            let new_pos = new_layout.children[0].layout.location.y;
            if (old_pos - new_pos).abs() > 0.1 {
                log::info!("Content container moved: old_y={:.1}, new_y={:.1}, delta={:.1}", old_pos, new_pos, new_pos - old_pos);
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
        let visible_children: Vec<_> = style.children.iter()
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

    /// Update the widget with the current layout.
    fn update_widget(&mut self, layout_node: &LayoutNode) {
        log::debug!("Updating root widget... ({} keyboard events, {} mouse buttons)", self.info.keys.len(), self.info.buttons.len());
        let context = self.context();
        self.update.insert(self.widget.as_mut().unwrap().update(
            layout_node,
            context,
            &mut self.info,
        ));
    }

    /// Render a frame to the screen.
    fn render_frame(&mut self, layout_node: &LayoutNode, event_loop: &ActiveEventLoop) {
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
            // Clear both DRAW and FORCE flags after successful rendering
            // FORCE should only trigger one render, not continuous rendering
            self.update
                .set(self.update.get() & !(Update::DRAW | Update::FORCE));
        } else {
            // Rendering failed - check if it's due to invalid surface size
            // If so, clear DRAW flag to prevent infinite loop (we'll retry when surface is configured)
            let surface_size = match &self.surface {
                Some(crate::vgi::Surface::Winit(_)) => {
                    // For Winit, get size from window
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
                // Clear DRAW flag but keep FORCE flag so we retry once surface is ready
                self.update.set(self.update.get() & !Update::DRAW);
            } else {
                // Other error - keep DRAW flag for retry, but clear FORCE to prevent infinite loop
                log::debug!("render_frame() failed - keeping DRAW flag for retry, clearing FORCE");
                self.update.set(self.update.get() & !Update::FORCE);
                log::debug!("Rendering failed, keeping DRAW flag for retry");
            }
        }
    }

    /// Render the main widget content.
    fn render_widget(&mut self, layout_node: &LayoutNode) -> Duration {
        log::debug!("Rendering root widget...");
        let start = Instant::now();

        let context = self.context();
        // Use unified Graphics API that works with both Vello and Hybrid backends
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

    /// Render postfix content (overlays, popups).
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
        if let Some((menu, position)) = context.menu_manager.get_active_menu() {
            if let Some(mut graphics) = graphics_from_scene(&mut self.scene) {
                let cursor_pos = self.info.cursor_pos.map(|p| vello::kurbo::Point::new(p.x, p.y));
                crate::menu::render_context_menu(
                    &mut *graphics,
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

    /// Render the scene to the surface, returning render times if successful.
    fn render_to_surface(
        &mut self,
        render_start: Instant,
        scene_reset_time: Duration,
        widget_render_time: Duration,
        postfix_render_time: Duration,
        event_loop: &ActiveEventLoop,
    ) -> Option<RenderTimes> {
        log::debug!("render_to_surface() called");

        // Don't render until async initialization is complete
        if !self.async_init_complete.load(Ordering::Relaxed) {
            log::warn!("Async initialization not complete. Skipping render.");
            return None;
        }

        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => {
                log::warn!("Renderer not initialized. Skipping render.");
                return None;
            },
        };

        let gpu_context = match self.gpu_context.as_ref() {
            Some(ctx) => ctx,
            None => {
                log::warn!("GPU context not initialized. Skipping render.");
                return None;
            },
        };

        let devices = gpu_context.enumerate_devices();
        if devices.is_empty() {
            log::warn!("No devices found. Skipping render.");
            return None;
        }

        let device_handle = (self.config.render.device_selector)(devices);

        // Get surface (must exist for rendering)
        let surface = match self.surface.as_mut() {
            Some(s) => s,
            None => {
                log::warn!("Surface not initialized. Skipping render.");
                return None;
            },
        };

        // On Wayland, reconfigure the surface if compositor requested it
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
                log::debug!("Wayland reconfigure requested...");
                if let Err(e) = wayland_surface.configure_surface(
                    &device_handle.device,
                    wayland_surface.format(),
                    present_mode,
                ) {
                    log::warn!("Wayland reconfigure failed: {}", e);
                } else {
                    log::debug!(
                        "Wayland surface reconfigured to {}x{}",
                        wayland_surface.size().0,
                        wayland_surface.size().1
                    );
                }
            }
            if !wayland_surface.is_configured() {
                log::warn!("Wayland surface not yet configured. Skipping render.");
                return None;
            }
        }

        // Get window size from surface (works for both Winit and Wayland)
        // For Winit, we need to get size from the window since Surface::size() returns (0, 0)
        let (width, height) = match &*surface {
            crate::vgi::Surface::Winit(_) => {
                // For Winit, get size from window
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
        log::debug!("Surface size: {}x{}", width, height);

        // Avoid doing event dispatch here for Wayland to prevent blocking before first frame.
        // Let the outer update() drive Wayland dispatch cadence.
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

        // Ensure Wayland surface is configured before acquiring texture
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        if let crate::vgi::Surface::Wayland(ref mut wayland_surface) = &mut *surface {
            // Proactively configure once after first configure to guarantee swapchain is ready.
            if wayland_surface.is_configured() && wayland_surface.requires_reconfigure() {
                let present_mode = match self.config.render.present_mode {
                    wgpu_types::PresentMode::AutoVsync => vello::wgpu::PresentMode::AutoVsync,
                    wgpu_types::PresentMode::AutoNoVsync => vello::wgpu::PresentMode::AutoNoVsync,
                    wgpu_types::PresentMode::Immediate => vello::wgpu::PresentMode::Immediate,
                    wgpu_types::PresentMode::Fifo => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::FifoRelaxed => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::Mailbox => vello::wgpu::PresentMode::Mailbox,
                };
                log::debug!("Wayland proactive configure before get_current_texture...");
                if let Err(e) = wayland_surface.configure_surface(
                    &device_handle.device,
                    wayland_surface.format(),
                    present_mode,
                ) {
                    log::warn!("Wayland proactive configure failed: {}", e);
                } else {
                    log::debug!(
                        "Wayland proactive configure OK ({}x{})",
                        wayland_surface.size().0,
                        wayland_surface.size().1
                    );
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
        log::debug!("Getting surface texture...");
        let surface_texture = match surface.get_current_texture() {
            Ok(texture) => {
                log::debug!("Successfully got surface texture");
                texture
            },
            Err(e) => {
                log::warn!("Failed to get surface texture: {}, skipping render", e);
                return None;
            },
        };
        let surface_get_time = surface_get_start.elapsed();

        // Pre-present notification (only for winit windows)
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }

        log::debug!("Rendering scene to surface ({}x{})...", width, height);
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
            log::warn!("Failed to render to surface: {}, skipping present", e);
            return None;
        }
        log::debug!("Successfully rendered scene to surface");
        let gpu_render_time = gpu_render_start.elapsed();

        let mut encoder = device_handle
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Surface Blit Encoder"),
            });

        let surface_view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        if let Err(err) = surface.blit_render_view(
            &device_handle.device,
            &mut encoder,
            &render_view,
            &surface_view,
        ) {
            log::error!("Failed to composite render target: {}", err);
            return None;
        }

        device_handle.queue.submit([encoder.finish()]);

        log::debug!("Presenting surface ({}x{})...", width, height);
        let present_start = Instant::now();

        // For Winit surfaces, we need to present the SurfaceTexture directly
        // The Surface::present() method is a no-op for Winit
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

        log::debug!("Successfully presented surface");
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
            let platform = Platform::detect();
            if platform == Platform::Wayland {
                // For Wayland, trigger update via event loop or directly
                // Since there's no winit window, we need to ensure update() is called
                // The event loop should call update() periodically, but we can also trigger it here
                // For now, rely on the event loop calling update() - it should work even without windows
            } else if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }

        if self.update.get().intersects(Update::EXIT) {
            event_loop.exit();
            return;
        }

        // CRITICAL: Don't clear DRAW or FORCE flags here - they're cleared after successful rendering
        // Only clear other flags that have been processed
        let flags_to_clear = self.update.get() & !(Update::DRAW | Update::FORCE);
        if flags_to_clear.bits() != 0 {
            // Preserve DRAW and FORCE flags - they're cleared in render_frame() after successful rendering
            self.update
                .set(self.update.get() & (Update::DRAW | Update::FORCE));
        }
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
    T: Theme + Clone,
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    /// Initialize heavy components asynchronously in the background
    fn initialize_async(&mut self, _event_loop: &ActiveEventLoop) {
        log::debug!("Starting async initialization...");

        // Create GpuContext first (creates Instance)
        // This follows GPUI's BladeContext pattern - create Instance before surfaces
        let mut gpu_context = match GpuContext::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                log::error!("Failed to create GPU context: {}", e);
                panic!("Failed to create GPU context: {}", e);
            },
        };

        // Detect platform
        let platform = Platform::detect();
        log::info!("Detected platform: {:?}", platform);

        // Create surface using GpuContext's Instance
        // For Wayland: Create Wayland surface with wgpu surface using GpuContext's Instance
        // For Winit: Create Winit surface using GpuContext's Instance
        self.create_surface(&gpu_context);

        // Request adapter with surface (for Wayland compatibility)
        let adapter = if platform == Platform::Wayland {
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            {
                if let Some(ref surface) = self.surface {
                    if let crate::vgi::Surface::Wayland(wayland_surf) = surface {
                        if let Some(ref wgpu_surface) = wayland_surf.wgpu_surface {
                            gpu_context.request_adapter_with_surface(wgpu_surface)
                        } else {
                            log::warn!("Wayland surface has no wgpu surface, falling back to adapter enumeration");
                            None
                        }
                    } else {
                        None
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

        // Create device from adapter (or from first adapter if no surface adapter)
        let device_handle = match if let Some(adapter) = adapter {
            gpu_context.create_device_from_adapter(&adapter)
        } else {
            // Fallback: create device from first adapter
            gpu_context.create_device_from_first_adapter(vello::wgpu::Backends::PRIMARY)
        } {
            Ok(handle) => handle,
            Err(e) => {
                log::error!("Failed to create device: {}", e);
                panic!("Failed to create device: {}", e);
            },
        };

        // Store device in GpuContext
        let device_handle_ref = {
            gpu_context.add_device(device_handle);
            // Get reference from GpuContext (we just added it, so it's the last one)
            let devices = gpu_context.enumerate_devices();
            devices.last().expect("Device should have been added")
        };

        // Create renderer with device
        self.create_renderer(device_handle_ref);

        // Configure surface (both Wayland and Winit need configuration)
        match &mut self.surface {
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            Some(crate::vgi::Surface::Wayland(ref mut wayland_surface)) => {
                // Get surface format from renderer options or use default
                let surface_format = wayland_surface.format();

                // Convert PresentMode from config to vello::wgpu::PresentMode
                let present_mode = match self.config.render.present_mode {
                    wgpu_types::PresentMode::AutoVsync => vello::wgpu::PresentMode::AutoVsync,
                    wgpu_types::PresentMode::AutoNoVsync => vello::wgpu::PresentMode::AutoNoVsync,
                    wgpu_types::PresentMode::Immediate => vello::wgpu::PresentMode::Immediate,
                    wgpu_types::PresentMode::Fifo => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::FifoRelaxed => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::Mailbox => vello::wgpu::PresentMode::Mailbox,
                };

                if let Err(e) = wayland_surface.configure_surface(
                    &device_handle_ref.device,
                    surface_format,
                    present_mode,
                ) {
                    log::error!("Failed to configure Wayland surface: {}", e);
                    panic!("Failed to configure Wayland surface: {}", e);
                }
                log::debug!("Wayland surface configured successfully");

                // Trigger initial redraw after surface is configured
                // For Wayland, we need to manually trigger rendering since there's no winit window
                self.update.insert(Update::FORCE | Update::DRAW);
            },
            Some(crate::vgi::Surface::Winit(ref mut winit_surface)) => {
                let window = self.window.as_ref().expect("Window should exist for Winit");
                let window_size = window.inner_size();

                let present_mode = match self.config.render.present_mode {
                    wgpu_types::PresentMode::AutoVsync => vello::wgpu::PresentMode::AutoVsync,
                    wgpu_types::PresentMode::AutoNoVsync => vello::wgpu::PresentMode::AutoNoVsync,
                    wgpu_types::PresentMode::Immediate => vello::wgpu::PresentMode::Immediate,
                    wgpu_types::PresentMode::Fifo => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::FifoRelaxed => vello::wgpu::PresentMode::Fifo,
                    wgpu_types::PresentMode::Mailbox => vello::wgpu::PresentMode::Mailbox,
                };

                if let Err(err) = winit_surface.configure(
                    &device_handle_ref.device,
                    &device_handle_ref.adapter,
                    window_size.width,
                    window_size.height,
                    present_mode,
                ) {
                    log::error!("Failed to configure Winit surface: {}", err);
                    panic!("Failed to configure Winit surface: {}", err);
                }

                if let Some(window) = &self.window {
                    window.set_visible(true);
                }
            },
            None => {
                log::error!("No surface available to configure");
                panic!("No surface available to configure");
            },
        }

        // Store GpuContext
        self.gpu_context = Some(Arc::new(gpu_context));
        self.async_init_complete.store(true, Ordering::Relaxed);

        log::debug!("Async initialization complete");

        // For Wayland, Update::FORCE | Update::DRAW was already set in configure handler above
        // For Winit, set Update::FORCE and request redraw
        if platform != Platform::Wayland {
            self.update.set(Update::FORCE);
            if let Some(window) = &self.window {
                log::debug!("Requesting initial redraw for winit window");
                window.request_redraw();
            }
        } else {
            log::debug!("Wayland: Update flags should already be set from configure handler");
        }
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
    fn build_renderer_options(config: &MayConfig<T>) -> RendererOptions {
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

    /// Handle a window event.
    fn handle_window_event(&mut self, event: WindowEvent, event_loop: &ActiveEventLoop) {
        match event {
            WindowEvent::Resized(new_size) => self.handle_resize(new_size, event_loop),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // Re-query physical size from the window, then derive logical size.
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
            WindowEvent::RedrawRequested => self.update_internal(event_loop),
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
            WindowEvent::Destroyed => log::info!("Window destroyed! Exiting..."),
            _ => (),
        }
    }

    /// Handle window resize event.
    fn handle_resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        _event_loop: &ActiveEventLoop,
    ) {
        if new_size.width == 0 || new_size.height == 0 {
            log::debug!("Window size is 0x0, ignoring resize event.");
            return;
        }

        log::info!("Window resized to {}x{}", new_size.width, new_size.height);

        // Resize the surface using physical pixels.
        if let Some(surface) = &mut self.surface {
            if let Err(e) = surface.resize(new_size.width, new_size.height) {
                log::error!("Failed to resize surface: {}", e);
            }
        }

        // Convert to logical size for layout/hit testing.
        let scale_factor = self
            .window
            .as_ref()
            .map(|w| w.scale_factor())
            .unwrap_or(1.0);
        let logical_size = new_size.to_logical::<f64>(scale_factor);

        self.update_window_node_size(logical_size.width as u32, logical_size.height as u32);
        self.info.size = Vector2::new(logical_size.width, logical_size.height);
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
            // Request exit - winit will handle window destruction
            // Don't clear window/surface references here as winit needs them for cleanup
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
                },
                PhysicalKey::Code(KeyCode::Escape) => {
                    // Handle ESC key for modal overlays
                },
                _ => {},
            }
        }

        let app_event = AppKeyEvent::from_winit(&event);
        self.info.keys.push((device_id, app_event));
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
        // Context Menu Logic
        if state == ElementState::Pressed {
             let context = self.context();
             if context.menu_manager.is_open() {
                 if let Some((menu, position)) = context.menu_manager.get_active_menu() {
                     if let Some(cursor_pos) = self.info.cursor_pos {
                         let cursor = vello::kurbo::Point::new(cursor_pos.x, cursor_pos.y);
                         if let Some(action) = crate::menu::handle_click(&menu, position, cursor) {
                             action();
                             context.menu_manager.close_context_menu();
                             self.update.insert(Update::DRAW);
                             return;
                         } else {
                             context.menu_manager.close_context_menu();
                             self.update.insert(Update::DRAW);
                             return;
                         }
                     }
                 }
             }
        }

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
                }
            };

            let devices = gpu_context.enumerate_devices();
            if devices.is_empty() {
                log::error!("No GPU devices available");
                continue;
            }
            let device_handle = (self.config.render.device_selector)(devices);

            let platform = crate::platform::Platform::detect();

            // Create window and surface based on platform
            let (window, mut surface) = match platform {
                #[cfg(all(target_os = "linux", feature = "wayland"))]
                crate::platform::Platform::Wayland => {
                    // Native Wayland: create only the Wayland surface, no Winit window
                    let surface = crate::platform::create_surface_blocking(
                        crate::platform::Platform::Wayland,
                        None, // No Winit window for native Wayland
                        req.size.0,
                        req.size.1,
                        &req.title,
                        Some(gpu_context),
                    ).expect("Failed to create Wayland surface");
                    (None, surface)
                }
                _ => {
                    // X11/Other: create Winit window and Winit surface
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
                        }
                    };

                    let surface = crate::platform::create_surface_blocking(
                        crate::platform::Platform::Winit,
                        Some(window.clone()),
                        req.size.0,
                        req.size.1,
                        &req.title,
                        Some(gpu_context),
                    ).expect("Failed to create Winit surface");

                    (Some(window), surface)
                }
            };

            // Configure the surface before rendering
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
                }
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
                }
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
                }
            };

            // Determine actual render dimensions (scaled for Wayland HiDPI)
            // Layout stays at logical size, buffer is at physical size
            #[cfg(all(target_os = "linux", feature = "wayland"))]
            let (render_width, render_height, popup_scale_factor) = if matches!(platform, crate::platform::Platform::Wayland) {
                (req.size.0 * 1, req.size.1 * 1, 1.0)
            } else {
                (req.size.0, req.size.1, 1.0)
            };
            #[cfg(not(all(target_os = "linux", feature = "wayland")))]
            let (render_width, render_height, popup_scale_factor) = (req.size.0, req.size.1, 1.0f32);

            // Setup Taffy - use LOGICAL size for layout
            let mut taffy = TaffyTree::new();
            let root_node = taffy.new_leaf(Style {
                size: Size {
                    width: Dimension::length(req.size.0 as f32),
                    height: Dimension::length(req.size.1 as f32),
                },
                ..Default::default()
            }).unwrap();

            // Create AppInfo for this window - use LOGICAL size
            let info = AppInfo {
                diagnostics: self.info.diagnostics.clone(),
                font_context: self.info.font_context.clone(),
                size: Vector2::new(req.size.0 as f64, req.size.1 as f64),
                focus_manager: self.info.focus_manager.clone(),
                ..Default::default()
            };

            // Scene uses PHYSICAL (scaled) size
            let scene = Scene::new(self.config.render.backend.clone(), render_width, render_height);

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

            // For Winit windows, we index by window ID
            // For native Wayland, we use a separate collection
            if let Some(ref win) = window {
                self.popup_windows.insert(win.id(), popup);
                win.request_redraw();
            } else {
                // For Wayland popups, use the wayland_popups collection
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

    /// Render all native Wayland popups
    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn render_wayland_popups(&mut self) {
        use crate::app::AppContext;
        use crate::vgi::graphics_from_scene;

        if self.wayland_popups.is_empty() {
            return;
        }

        let gpu_context = match self.gpu_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => return,
        };

        let devices = gpu_context.enumerate_devices();
        if devices.is_empty() {
            return;
        }
        let device_handle = (self.config.render.device_selector)(devices);

        let mut to_remove: Vec<u32> = Vec::new();

        for (popup_id, popup) in self.wayland_popups.iter_mut() {
            // Drive Wayland events so we notice compositor-side closes.
            if popup.surface.needs_event_dispatch() {
                if let Err(err) = popup.surface.dispatch_events() {
                    log::info!(
                        "Wayland popup requested close (id={}): {}",
                        popup_id,
                        err
                    );
                    to_remove.push(*popup_id);
                    continue;
                }

                // If dispatch succeeded but the surface requested close, remove it.
                if let crate::vgi::Surface::Wayland(ref wayland_surface) = popup.surface {
                    if wayland_surface.should_close() {
                        log::info!(
                            "Wayland popup requested close after dispatch (id={})",
                            popup_id
                        );
                        to_remove.push(*popup_id);
                        continue;
                    }
                }
            }

            // Reconfigure popup surface if the compositor requested it (or if not yet configured).
            if let crate::vgi::Surface::Wayland(ref mut wayland_surface) = popup.surface {
                if wayland_surface.requires_reconfigure() || !wayland_surface.is_configured() {
                    let (new_w, new_h) = wayland_surface.size();
                    if let Err(e) = wayland_surface.configure_surface(
                        &device_handle.device,
                        wayland_surface.format(),
                        self.config.render.present_mode.into(),
                    ) {
                        log::error!(
                            "Failed to reconfigure Wayland popup surface (id={}): {}",
                            popup_id,
                            e
                        );
                        to_remove.push(*popup_id);
                        continue;
                    }

                    // Update logical sizes to match new compositor-requested dimensions.
                    popup.info.size = Vector2::new(new_w as f64, new_h as f64);
                    let _ = popup.taffy.set_style(
                        popup.root_node,
                        Style {
                            size: Size {
                                width: Dimension::length(new_w as f32),
                                height: Dimension::length(new_h as f32),
                            },
                            ..Default::default()
                        },
                    );
                }
            }

            // Use physical size for rendering (scaled)
            let physical_width = (popup.info.size.x * popup.scale_factor as f64) as u32;
            let physical_height = (popup.info.size.y * popup.scale_factor as f64) as u32;

            // 1. Layout (at logical size)
            let style = popup.widget.layout_style();
            let _ = popup.taffy.set_children(popup.root_node, &[]);
            
            // Use helper to build full layout tree
            if let Err(e) = layout_widget_tree(&mut popup.taffy, popup.root_node, &style) {
                eprintln!("Failed to build popup layout tree: {}", e);
                continue;
            }
            
            let _ = popup.taffy.compute_layout(popup.root_node, Size::MAX_CONTENT);

            // 2. Render to Scene (at physical size, with scale transform)
            let mut builder = Scene::new(popup.config.render.backend.clone(), physical_width, physical_height);
            {
                let mut graphics = match graphics_from_scene(&mut builder) {
                    Some(g) => g,
                    None => continue,
                };

                // Push a layer with scale transform for HiDPI
                let scale_transform = vello::kurbo::Affine::scale(popup.scale_factor as f64);
                let full_rect = vello::kurbo::Rect::new(0.0, 0.0, physical_width as f64, physical_height as f64);
                use vello::kurbo::Shape;
                graphics.push_layer(
                    vello::peniko::Mix::Normal,
                    1.0,
                    scale_transform,
                    &full_rect.to_path(0.1),
                );
                
                let child_count = popup.taffy.child_count(popup.root_node);
                if child_count > 0 {
                    let widget_node = popup.taffy.child_at_index(popup.root_node, 0).unwrap();
                    match collect_layout_tree(&popup.taffy, widget_node, &style, 0.0, 0.0) {
                        Ok(layout_node) => {
                            let context = AppContext::new(
                                self.update.clone(),
                                self.info.diagnostics.clone(),
                                gpu_context.clone(),
                                self.info.focus_manager.clone(),
                                self.menu_manager.clone(),
                                self.popup_manager.clone(),
                                self.settings.clone(),
                            );

                            popup.widget.render(graphics.as_mut(), &mut popup.config.theme, &layout_node, &mut popup.info, context);
                        },
                        Err(e) => eprintln!("Failed to collect popup layout: {}", e),
                    }
                }

                // Pop the scale layer
                graphics.pop_layer();
            }

            // Render to surface
            let render_view = match popup.surface.create_render_view(&device_handle.device, physical_width, physical_height) {
                Ok(view) => view,
                Err(e) => {
                    log::error!("Failed to create render view for Wayland popup: {}", e);
                    continue;
                }
            };

            if let Err(e) = popup.renderer.render_to_view(
                &device_handle.device,
                &device_handle.queue,
                &builder,
                &render_view,
                &RenderParams {
                    base_color: popup.config.theme.window_background(),
                    width: physical_width,
                    height: physical_height,
                    antialiasing_method: popup.config.render.antialiasing,
                },
            ) {
                log::error!("Failed to render Wayland popup scene: {}", e);
                continue;
            }

            // Blit to surface
            let mut encoder = device_handle.device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Wayland Popup Blit Encoder"),
            });

            let surface_texture = match popup.surface.get_current_texture() {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to get Wayland popup surface texture: {}", e);
                    continue;
                }
            };

            let surface_view = surface_texture.texture.create_view(&TextureViewDescriptor::default());

            if let Err(e) = popup.surface.blit_render_view(&device_handle.device, &mut encoder, &render_view, &surface_view) {
                log::error!("Failed to blit Wayland popup surface: {}", e);
                continue;
            }

            device_handle.queue.submit(Some(encoder.finish()));
            surface_texture.present();
        }

        for popup_id in to_remove {
            self.wayland_popups.remove(&popup_id);
        }
    }
}

impl<T, W, S, F> ApplicationHandler for AppHandler<T, W, S, F>
where
    T: Theme + Clone,
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
            log::debug!("Popup window event: {:?} for window: {:?}", event, window_id);
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
                    let _ = popup.taffy.compute_layout(popup.root_node, Size::MAX_CONTENT);

                    // 2. Render to Scene
                    let mut builder = Scene::new(popup.config.render.backend.clone(), width, height);
                    {
                        let mut graphics = graphics_from_scene(&mut builder).unwrap();
                        let child_count = popup.taffy.child_count(popup.root_node);
                        if child_count > 0 {
                            let widget_node = popup.taffy.child_at_index(popup.root_node, 0).unwrap();
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
                                    popup.widget.render(
                                        graphics.as_mut(),
                                        &mut popup.config.theme,
                                        &layout_node,
                                        &mut popup.info,
                                        context,
                                    );
                                }
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

                            let render_view = match popup.surface.create_render_view(&device_handle.device, width, height) {
                                Ok(view) => view,
                                Err(e) => {
                                    log::error!("Failed to create render view for popup: {}", e);
                                    return;
                                }
                            };

                            if let Err(e) = popup.renderer.render_to_view(
                                &device_handle.device,
                                &device_handle.queue,
                                &builder,
                                &render_view,
                                &RenderParams {
                                    base_color: popup.config.theme.window_background(),
                                    width,
                                    height,
                                    antialiasing_method: popup.config.render.antialiasing,
                                },
                            ) {
                                log::error!("Failed to render popup scene: {}", e);
                                return;
                            }

                            let mut encoder = device_handle.device.create_command_encoder(&CommandEncoderDescriptor {
                                label: Some("Popup Surface Blit Encoder"),
                            });

                            let surface_texture = match popup.surface.get_current_texture() {
                                Ok(t) => t,
                                Err(e) => {
                                    log::error!("Failed to get popup surface texture: {}", e);
                                    return;
                                }
                            };

                            let surface_view = surface_texture.texture.create_view(&TextureViewDescriptor::default());

                            if let Err(e) = popup.surface.blit_render_view(&device_handle.device, &mut encoder, &render_view, &surface_view) {
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
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    let physical = popup
                        .window
                        .as_ref()
                        .map(|w| w.inner_size())
                        .unwrap_or(winit::dpi::PhysicalSize::new(0, 0));
                    let _ = popup.surface.resize(physical.width, physical.height);
                    popup.renderer.update_render_target_size(physical.width, physical.height);

                    let logical = physical.to_logical::<f64>(scale_factor);
                    let _ = popup.taffy.set_style(popup.root_node, Style {
                        size: Size {
                            width: Dimension::length(logical.width as f32),
                            height: Dimension::length(logical.height as f32),
                        },
                        ..Default::default()
                    });
                    popup.config.window.size = Vector2::new(logical.width, logical.height);
                    if let Some(ref win) = popup.window {
                        win.request_redraw();
                    }
                }
                WindowEvent::Resized(size) => {
                    let _ = popup.surface.resize(size.width, size.height);
                    popup.renderer.update_render_target_size(size.width, size.height);

                    let scale_factor = popup
                        .window
                        .as_ref()
                        .map(|w| w.scale_factor())
                        .unwrap_or(popup.scale_factor as f64);
                    let logical = size.to_logical::<f64>(scale_factor);

                    let _ = popup.taffy.set_style(popup.root_node, Style {
                        size: Size {
                            width: Dimension::length(logical.width as f32),
                            height: Dimension::length(logical.height as f32),
                        },
                        ..Default::default()
                    });
                    popup.config.window.size = Vector2::new(logical.width, logical.height);
                    if let Some(ref win) = popup.window {
                        win.request_redraw();
                    }
                }
                _ => {}
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
fn collect_layout_tree(taffy: &TaffyTree, node: NodeId, style: &StyleNode, parent_x: f32, parent_y: f32) -> TaffyResult<LayoutNode> {
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
            children.push(collect_layout_tree(taffy, child_node, child_style, child_content_x, child_content_y)?);
            taffy_index += 1;
        }
    }
    
    Ok(LayoutNode {
        layout: absolute_layout,
        children,
    })
}
