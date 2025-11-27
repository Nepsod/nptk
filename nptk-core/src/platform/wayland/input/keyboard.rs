#![cfg(target_os = "linux")]

//! Keyboard input handling.

use std::ffi::c_void;
use std::os::unix::io::AsRawFd;
use std::ptr;

use wayland_client::protocol::wl_keyboard;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use super::super::client::SharedState;
use super::super::events::{InputEvent, KeyboardEvent};
use super::super::shell::WaylandClientState;

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter {
                surface,
                serial: _,
                keys: _,
            } => {
                let key = surface.id().protocol_id();
                log::info!("Wayland keyboard focus entered for surface {}", key);
                state.shared.set_focused_surface(Some(key));
                if let Some(surface) = state.shared.get_surface(key) {
                    surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Enter));
                    surface.request_redraw();
                }
            },
            wl_keyboard::Event::Leave { serial: _, .. } => {
                let focused = state.shared.get_focused_surface_key();
                if let Some(key) = focused {
                    if let Some(surface) = state.shared.get_surface(key) {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Leave));
                        surface.request_redraw();
                    }
                }
                state.shared.set_focused_surface(None);
            },
            wl_keyboard::Event::Key {
                serial: _,
                time: _,
                key,
                state: key_state,
            } => {
                let focused = state.shared.get_focused_surface_key();
                log::debug!("Wayland keyboard key event: key={}, state={:?}, focused_surface={:?}", key, key_state, focused);
                
                // If no focused surface, try to find any registered surface
                // Some compositors send keyboard events even without explicit focus
                let target_surface = if let Some(key_surface) = focused {
                    Some(key_surface)
                } else {
                    // Try to find the first registered surface as a fallback
                    let surfaces_map = state.shared.surfaces().lock().unwrap();
                    let first_surface_key = surfaces_map.keys().next().copied();
                    drop(surfaces_map);
                    if let Some(surface_key) = first_surface_key {
                        log::debug!("No focused surface, using first registered surface {} as fallback", surface_key);
                        Some(surface_key)
                    } else {
                        None
                    }
                };
                
                if let Some(key_surface) = target_surface {
                    if let Some(surface) = state.shared.get_surface(key_surface) {
                        if let Ok(actual_state) = key_state.into_result() {
                            log::debug!("Pushing keyboard event to surface {}", key_surface);
                            surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Key {
                                keycode: key,
                                state: actual_state,
                            }));
                            surface.request_redraw();
                        } else {
                            log::debug!("Failed to convert key_state to result");
                        }
                    } else {
                        log::debug!("Surface {} not found", key_surface);
                    }
                } else {
                    log::debug!("No focused surface and no registered surfaces for keyboard event");
                }
            },
            wl_keyboard::Event::Modifiers {
                serial: _,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                if let Some(key_surface) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key_surface) {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Modifiers {
                            mods_depressed,
                            mods_latched,
                            mods_locked,
                            group,
                        }));
                        surface.request_redraw();
                    }
                }
            },
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                if let Some(key_surface) = state.shared.get_focused_surface_key() {
                    if let Some(surface) = state.shared.get_surface(key_surface) {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::RepeatInfo {
                            rate,
                            delay,
                        }));
                        surface.request_redraw();
                    }
                }
            },
            wl_keyboard::Event::Keymap { format, fd, size } => {
                log::debug!("Wayland keyboard keymap received: format={:?}, size={}", format, size);
                
                // Read the keymap from the file descriptor using mmap
                // This avoids ownership issues with OwnedFd
                let raw_fd = fd.as_raw_fd();
                let keymap_string = unsafe {
                    let mapped = libc::mmap(
                        ptr::null_mut(),
                        size as usize,
                        libc::PROT_READ,
                        libc::MAP_PRIVATE,
                        raw_fd,
                        0,
                    );
                    
                    if mapped == libc::MAP_FAILED {
                        let errno = *libc::__errno_location();
                        log::warn!("Failed to mmap keymap fd: errno={}", errno);
                        return;
                    }
                    
                    // Create a slice from the mapped memory and convert to String
                    let slice = std::slice::from_raw_parts(mapped as *const u8, size as usize);
                    let result = match String::from_utf8(slice.to_vec()) {
                        Ok(s) => s,
                        Err(e) => {
                            log::warn!("Failed to convert keymap to UTF-8: {}", e);
                            libc::munmap(mapped, size as usize);
                            return;
                        }
                    };
                    
                    // Unmap the memory
                    libc::munmap(mapped, size as usize);
                    
                    result
                };
                
                log::debug!("Read keymap string ({} bytes)", keymap_string.len());
                
                // Send the keymap to all surfaces (keymap is global for the keyboard)
                let surfaces_map = state.shared.surfaces().lock().unwrap();
                for (surface_key, surface_weak) in surfaces_map.iter() {
                    if let Some(surface) = surface_weak.upgrade() {
                        surface.push_input_event(InputEvent::Keyboard(KeyboardEvent::Keymap {
                            keymap_string: keymap_string.clone(),
                        }));
                        surface.request_redraw();
                    }
                }
                // Original fd will be closed automatically when it goes out of scope
            },
            _ => {},
        }
    }
}

