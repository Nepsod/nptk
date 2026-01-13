#![cfg(all(target_os = "linux", feature = "wayland"))]

use super::{collect_layout_tree, layout_widget_tree, AppHandler};
use crate::app::context::AppContext;
use crate::app::info::AppKeyEvent;
use crate::platform::wayland::events::{InputEvent, KeyboardEvent, PointerEvent};
use crate::platform::Platform;
use crate::vgi::surface::SurfaceTrait;
use crate::vgi::Scene;
use crate::widget::Widget;
use nalgebra::Vector2;
use nptk_theme::theme::Theme;
use taffy::prelude::*;
use vello::wgpu::{CommandEncoderDescriptor, TextureViewDescriptor};
use vello::RenderParams;
use wayland_client::protocol::{wl_keyboard, wl_pointer};
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{
    Key, KeyCode, ModifiersState, NamedKey, NativeKey, NativeKeyCode, PhysicalKey,
};

impl<W, S, F> AppHandler<W, S, F>
where
    W: Widget,
    F: Fn(AppContext, S) -> W,
{
    pub(super) fn process_wayland_input_events(&mut self) {
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
                InputEvent::Keyboard(key_event) => match key_event {
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

                        let (physical_key, text) = if self.xkb_keymap.is_ready() {
                            use xkbcommon_dl::xkb_key_direction;
                            let direction = match element_state {
                                ElementState::Pressed => xkb_key_direction::XKB_KEY_DOWN,
                                ElementState::Released => xkb_key_direction::XKB_KEY_UP,
                            };
                            let xkb_keycode = keycode + 8;
                            let keysym = self.xkb_keymap.keycode_to_keysym(xkb_keycode, direction);
                            log::debug!(
                                "XKB keycode {} (direction={:?}) -> keysym {:?}",
                                xkb_keycode,
                                direction,
                                keysym
                            );
                            let utf8_text = if element_state == ElementState::Pressed {
                                use xkbcommon_dl::xkb_key_direction;
                                self.xkb_keymap
                                    .keycode_to_utf8(xkb_keycode, xkb_key_direction::XKB_KEY_DOWN)
                            } else {
                                None
                            };

                            let physical = if let Some(ks) = keysym {
                                if keycode == 68 && (ks == 0xFFBF || ks == 65471) {
                                    log::debug!(
                                        "Detected F10: keycode=68, keysym=0xFFBF, using F10 directly"
                                    );
                                    PhysicalKey::Code(KeyCode::F10)
                                } else {
                                    let mapped = Self::keysym_to_physical_key(ks);
                                    log::debug!(
                                        "XKB keysym {} (0x{:X}) -> physical_key {:?}",
                                        ks,
                                        ks,
                                        mapped
                                    );
                                    match &mapped {
                                        PhysicalKey::Unidentified(_) => {
                                            log::debug!("Keysym {} (0x{:X}) not recognized, falling back to hardcoded mapping (keycode={}, evdev={})", ks, ks, keycode, Self::normalize_wayland_keycode(keycode));
                                            let evdev = Self::normalize_wayland_keycode(keycode);
                                            let fallback =
                                                Self::map_wayland_physical_key(evdev, keycode);
                                            log::debug!(
                                                "Hardcoded mapping: evdev {} -> {:?}",
                                                evdev,
                                                fallback
                                            );
                                            fallback
                                        },
                                        _ => mapped,
                                    }
                                }
                            } else {
                                log::debug!(
                                    "XKB keysym lookup failed, falling back to hardcoded mapping"
                                );
                                let evdev = Self::normalize_wayland_keycode(keycode);
                                Self::map_wayland_physical_key(evdev, keycode)
                            };

                            (physical, utf8_text)
                        } else {
                            log::debug!(
                                "XKB keymap not ready, using hardcoded mapping for keycode {}",
                                keycode
                            );
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
                            .unwrap_or_else(|| match physical_key {
                                PhysicalKey::Code(KeyCode::Escape) => Key::Named(NamedKey::Escape),
                                PhysicalKey::Code(KeyCode::Tab) => Key::Named(NamedKey::Tab),
                                PhysicalKey::Code(KeyCode::Backspace) => {
                                    Key::Named(NamedKey::Backspace)
                                },
                                PhysicalKey::Code(KeyCode::Enter) => Key::Named(NamedKey::Enter),
                                PhysicalKey::Code(KeyCode::Space) => Key::Named(NamedKey::Space),
                                PhysicalKey::Code(KeyCode::Delete) => Key::Named(NamedKey::Delete),
                                PhysicalKey::Code(KeyCode::Insert) => Key::Named(NamedKey::Insert),
                                PhysicalKey::Code(KeyCode::Home) => Key::Named(NamedKey::Home),
                                PhysicalKey::Code(KeyCode::End) => Key::Named(NamedKey::End),
                                PhysicalKey::Code(KeyCode::PageUp) => Key::Named(NamedKey::PageUp),
                                PhysicalKey::Code(KeyCode::PageDown) => {
                                    Key::Named(NamedKey::PageDown)
                                },
                                PhysicalKey::Code(KeyCode::ArrowUp) => {
                                    Key::Named(NamedKey::ArrowUp)
                                },
                                PhysicalKey::Code(KeyCode::ArrowDown) => {
                                    Key::Named(NamedKey::ArrowDown)
                                },
                                PhysicalKey::Code(KeyCode::ArrowLeft) => {
                                    Key::Named(NamedKey::ArrowLeft)
                                },
                                PhysicalKey::Code(KeyCode::ArrowRight) => {
                                    Key::Named(NamedKey::ArrowRight)
                                },
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
                                PhysicalKey::Code(KeyCode::ShiftLeft)
                                | PhysicalKey::Code(KeyCode::ShiftRight) => {
                                    Key::Named(NamedKey::Shift)
                                },
                                PhysicalKey::Code(KeyCode::ControlLeft)
                                | PhysicalKey::Code(KeyCode::ControlRight) => {
                                    Key::Named(NamedKey::Control)
                                },
                                PhysicalKey::Code(KeyCode::AltLeft)
                                | PhysicalKey::Code(KeyCode::AltRight) => Key::Named(NamedKey::Alt),
                                PhysicalKey::Code(KeyCode::SuperLeft)
                                | PhysicalKey::Code(KeyCode::SuperRight) => {
                                    Key::Named(NamedKey::Super)
                                },
                                _ => Key::Unidentified(NativeKey::Unidentified),
                            });

                        let app_event = AppKeyEvent {
                            physical_key,
                            logical_key,
                            text,
                            state: element_state,
                            repeat,
                        };

                        log::debug!(
                            "Wayland keyboard event: keycode={}, evdev={}, physical_key={:?}, state={:?}",
                            keycode,
                            evdev,
                            physical_key,
                            element_state
                        );

                        let keyboard_device = DeviceId::dummy();
                        self.info.keys.push((keyboard_device, app_event));
                        self.request_redraw();
                    },
                    KeyboardEvent::Modifiers {
                        mods_depressed,
                        mods_latched,
                        mods_locked,
                        group,
                    } => {
                        self.xkb_keymap.update_modifiers(
                            mods_depressed,
                            mods_latched,
                            mods_locked,
                            group,
                        );
                        self.update_wayland_modifiers_state();
                    },
                    KeyboardEvent::RepeatInfo { .. } => {},
                    KeyboardEvent::Keymap { keymap_string } => {
                        log::info!(
                            "Received keymap ({} bytes), updating XKB keymap manager",
                            keymap_string.len()
                        );
                        if let Err(e) = self.xkb_keymap.update_keymap(&keymap_string) {
                            log::warn!("Failed to update XKB keymap: {}", e);
                        } else {
                            log::info!(
                                "XKB keymap updated successfully, ready={}",
                                self.xkb_keymap.is_ready()
                            );
                        }
                    },
                },
                InputEvent::Touch(_touch_event) => {},
                InputEvent::Tablet(_tablet_event) => {},
            }
        }

        self.flush_wayland_scroll(&mut pending_scroll, &mut scroll_is_line, &mut axis_source);
    }

    fn flush_wayland_scroll(
        &mut self,
        pending: &mut Option<(f64, f64)>,
        scroll_is_line: &mut bool,
        axis_source: &mut Option<wayland_client::protocol::wl_pointer::AxisSource>,
    ) {
        use wayland_client::protocol::wl_pointer::AxisSource;

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

    fn normalize_wayland_keycode(keycode: u32) -> u32 {
        keycode
    }

    fn keysym_to_physical_key(keysym: u32) -> PhysicalKey {
        use xkbcommon_dl::keysyms;

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

    pub(super) fn render_wayland_popups(&mut self) {
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
            if popup.surface.needs_event_dispatch() {
                if let Err(err) = popup.surface.dispatch_events() {
                    log::info!("Wayland popup requested close (id={}): {}", popup_id, err);
                    to_remove.push(*popup_id);
                    continue;
                }

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

            let physical_width = (popup.info.size.x * popup.scale_factor as f64) as u32;
            let physical_height = (popup.info.size.y * popup.scale_factor as f64) as u32;

            let style = popup.widget.layout_style();
            let _ = popup.taffy.set_children(popup.root_node, &[]);

            if let Err(e) = layout_widget_tree(&mut popup.taffy, popup.root_node, &style) {
                eprintln!("Failed to build popup layout tree: {}", e);
                continue;
            }

            let _ = popup
                .taffy
                .compute_layout(popup.root_node, Size::MAX_CONTENT);

            let mut builder = Scene::new(
                popup.config.render.backend.clone(),
                physical_width,
                physical_height,
            );
            {
                let mut graphics = match graphics_from_scene(&mut builder) {
                    Some(g) => g,
                    None => continue,
                };

                let scale_transform = vello::kurbo::Affine::scale(popup.scale_factor as f64);
                let full_rect = vello::kurbo::Rect::new(
                    0.0,
                    0.0,
                    physical_width as f64,
                    physical_height as f64,
                );
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
                }

                graphics.pop_layer();
            }

            let render_view = match popup.surface.create_render_view(
                &device_handle.device,
                physical_width,
                physical_height,
            ) {
                Ok(view) => view,
                Err(e) => {
                    log::error!("Failed to create render view for Wayland popup: {}", e);
                    continue;
                },
            };

            let base_color = popup.config.theme_manager.read().unwrap()
                .access_theme(|theme| theme.window_background())
                .unwrap_or_else(|| vello::peniko::Color::WHITE);
            
            if let Err(e) = popup.renderer.render_to_view(
                &device_handle.device,
                &device_handle.queue,
                &builder,
                &render_view,
                &RenderParams {
                    base_color,
                    width: physical_width,
                    height: physical_height,
                    antialiasing_method: popup.config.render.antialiasing,
                },
            ) {
                log::error!("Failed to render Wayland popup scene: {}", e);
                continue;
            }

            let mut encoder =
                device_handle
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Wayland Popup Blit Encoder"),
                    });

            let surface_texture = match popup.surface.get_current_texture() {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Failed to get Wayland popup surface texture: {}", e);
                    continue;
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
                log::error!("Failed to blit Wayland popup surface: {}", e);
                continue;
            }

            device_handle
                .queue
                .submit(std::iter::once(encoder.finish()));
            surface_texture.present();
        }

        for popup_id in to_remove {
            self.wayland_popups.remove(&popup_id);
        }
    }
}
