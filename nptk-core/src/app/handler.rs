#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::vgi::graphics_from_scene;
use crate::vgi::{DeviceHandle, GpuContext};
use crate::vgi::{Platform, Renderer, RendererOptions, Scene, Surface, SurfaceTrait};
use nalgebra::Vector2;
use taffy::{
    AvailableSpace, Dimension, NodeId, PrintTree, Size, Style, TaffyResult, TaffyTree,
    TraversePartialTree,
};
use vello::wgpu::{CommandEncoderDescriptor, TextureViewDescriptor};
use vello::{AaConfig, AaSupport, RenderParams};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::context::AppContext;
use crate::app::font_ctx::FontContext;
#[cfg(target_os = "linux")]
use crate::app::info::WindowIdentity;
use crate::app::info::{AppInfo, AppKeyEvent};
use crate::app::update::{Update, UpdateManager};
use crate::config::MayConfig;
use crate::layout::{LayoutNode, StyleNode};
use crate::plugin::PluginManager;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use crate::vgi::wayland_surface::{InputEvent, KeyboardEvent, PointerEvent};
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
        }
    }

    #[cfg(all(target_os = "linux", feature = "wayland"))]
    fn process_wayland_input_events(&mut self) {
        use crate::vgi::Platform;
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
                                let keysym = self.xkb_keymap.keycode_to_keysym(keycode, direction);
                                log::debug!("XKB keycode {} (direction={:?}) -> keysym {:?}", keycode, direction, keysym);
                                let utf8_text = if element_state == ElementState::Pressed {
                                    use xkbcommon_dl::xkb_key_direction;
                                    self.xkb_keymap.keycode_to_utf8(keycode, xkb_key_direction::XKB_KEY_DOWN)
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
                                .unwrap_or_else(|| Key::Unidentified(NativeKey::Unidentified));

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
        if keycode >= 8 {
            keycode - 8
        } else {
            keycode
        }
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
            self.info.diagnostics,
            self.gpu_context.clone().unwrap(),
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

        self.update_widget(&layout_node);

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
        // Use unified Graphics API that works with both Vello and Hybrid backends
        let mut graphics =
            graphics_from_scene(&mut self.scene).expect("Failed to create graphics from scene");
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
    T: Theme,
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
            crate::vgi::platform::create_surface_blocking(
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
            WindowEvent::CloseRequested => self.handle_close_request(event_loop),
            WindowEvent::RedrawRequested => self.update_internal(event_loop),
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
                self.info.diagnostics,
                Arc::new(GpuContext::new().expect("Failed to create GPU context")), // Temporary GPU context
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
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Drive app updates even if no winit window exists (Wayland-native path)
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        {
            if crate::vgi::Platform::detect() == crate::vgi::Platform::Wayland {
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
