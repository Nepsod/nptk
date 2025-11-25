#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::ffi::CString;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::os::raw::c_char;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use std::sync::Mutex;
#[cfg(all(target_os = "linux", feature = "wayland"))]
use xkbcommon_dl::{
    xkb_context, xkb_context_flags, xkb_key_direction, xkb_keymap, xkb_keymap_compile_flags,
    xkb_keymap_format, xkb_state, xkbcommon_handle, xkb_keycode_t,
};

#[cfg(all(target_os = "linux", feature = "wayland"))]
pub struct XkbKeymapManager {
    context: Mutex<Option<*mut xkb_context>>,
    keymap: Mutex<Option<*mut xkb_keymap>>,
    state: Mutex<Option<*mut xkb_state>>,
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
impl XkbKeymapManager {
    pub fn new() -> Result<Self, String> {
        let handle = xkbcommon_handle();
        let context = unsafe {
            (handle.xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS)
        };
        if context.is_null() {
            return Err("Failed to create XKB context".to_string());
        }

        Ok(Self {
            context: Mutex::new(Some(context)),
            keymap: Mutex::new(None),
            state: Mutex::new(None),
        })
    }

    pub fn update_keymap(&self, keymap_string: &str) -> Result<(), String> {
        let handle = xkbcommon_handle();
        let context_guard = self.context.lock().unwrap();
        let context = context_guard.ok_or("XKB context not available")?;
        if context.is_null() {
            return Err("XKB context is null".to_string());
        }

        // Convert string to CString for FFI
        // XKB keymaps are text files that may be null-terminated
        // Strip any trailing null bytes and ensure single null terminator
        let keymap_bytes = keymap_string.as_bytes();
        let mut keymap_vec = if keymap_bytes.ends_with(&[0]) {
            // Remove trailing null bytes
            let mut vec: Vec<u8> = keymap_bytes.to_vec();
            while vec.last() == Some(&0) {
                vec.pop();
            }
            vec
        } else {
            keymap_bytes.to_vec()
        };
        
        // Ensure null termination for CString
        keymap_vec.push(0);
        
        let keymap_cstr = CString::from_vec_with_nul(keymap_vec)
            .map_err(|e| format!("Failed to create CString from keymap: {}", e))?;

        let keymap = unsafe {
            (handle.xkb_keymap_new_from_string)(
                context,
                keymap_cstr.as_ptr(),
                xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
            )
        };

        if keymap.is_null() {
            return Err("Failed to create XKB keymap".to_string());
        }

        let state = unsafe { (handle.xkb_state_new)(keymap) };
        if state.is_null() {
            unsafe {
                (handle.xkb_keymap_unref)(keymap);
            }
            return Err("Failed to create XKB state".to_string());
        }

        drop(context_guard);

        // Clean up old keymap and state if they exist
        let mut keymap_guard = self.keymap.lock().unwrap();
        if let Some(old_keymap) = keymap_guard.take() {
            unsafe {
                (handle.xkb_keymap_unref)(old_keymap);
            }
        }
        *keymap_guard = Some(keymap);

        let mut state_guard = self.state.lock().unwrap();
        if let Some(old_state) = state_guard.take() {
            unsafe {
                (handle.xkb_state_unref)(old_state);
            }
        }
        *state_guard = Some(state);

        log::debug!("XKB keymap updated successfully");
        Ok(())
    }

    pub fn update_modifiers(
        &self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) {
        let state_guard = self.state.lock().unwrap();
        if let Some(state) = *state_guard {
            if !state.is_null() {
                unsafe {
                    let handle = xkbcommon_handle();
                    (handle.xkb_state_update_mask)(state, mods_depressed, mods_latched, mods_locked, 0, 0, group);
                }
            }
        }
    }

    pub fn keycode_to_keysym(&self, keycode: u32, direction: xkb_key_direction) -> Option<u32> {
        // Wayland keycodes are evdev scancodes + 8, which matches XKB keycodes
        let state_guard = self.state.lock().unwrap();
        state_guard.and_then(|state| {
            if state.is_null() {
                log::debug!("XKB state is null");
                None
            } else {
                unsafe {
                    let handle = xkbcommon_handle();
                    // Get keysym first (before updating state) to get the base keysym
                    // The keysym should be the same regardless of key direction for function keys
                    let keysym = (handle.xkb_state_key_get_one_sym)(state, keycode as xkb_keycode_t);
                    // Update the state with the key direction for modifier tracking
                    (handle.xkb_state_update_key)(
                        state,
                        keycode as xkb_keycode_t,
                        direction,
                    );
                    if keysym != 0 {
                        Some(keysym)
                    } else {
                        log::debug!("XKB keycode {} -> keysym 0 (no keysym)", keycode);
                        None
                    }
                }
            }
        })
    }

    pub fn keycode_to_utf8(&self, keycode: u32, direction: xkb_key_direction) -> Option<String> {
        // Wayland keycodes are evdev scancodes + 8, which matches XKB keycodes
        let state_guard = self.state.lock().unwrap();
        state_guard.and_then(|state| {
            if state.is_null() {
                None
            } else {
                unsafe {
                    let handle = xkbcommon_handle();
                    // Update state with key direction
                    (handle.xkb_state_update_key)(
                        state,
                        keycode as xkb_keycode_t,
                        direction,
                    );
                    let mut buffer = [0u8; 64];
                    let len = (handle.xkb_state_key_get_utf8)(
                        state,
                        keycode as xkb_keycode_t,
                        buffer.as_mut_ptr() as *mut c_char,
                        buffer.len(),
                    );
                    if len > 0 {
                        String::from_utf8(buffer[..len as usize].to_vec()).ok()
                    } else {
                        None
                    }
                }
            }
        })
    }

    pub fn keycode_to_keycode_name(&self, keycode: u32) -> Option<String> {
        // This would require keymap access, which is more complex
        // For now, return None as this is not critical
        None
    }

    pub fn is_ready(&self) -> bool {
        let keymap_guard = self.keymap.lock().unwrap();
        let state_guard = self.state.lock().unwrap();
        keymap_guard.is_some() && state_guard.is_some()
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
impl Drop for XkbKeymapManager {
    fn drop(&mut self) {
        let handle = xkbcommon_handle();
        unsafe {
            if let Some(state) = self.state.get_mut().unwrap().take() {
                if !state.is_null() {
                    (handle.xkb_state_unref)(state);
                }
            }
            if let Some(keymap) = self.keymap.get_mut().unwrap().take() {
                if !keymap.is_null() {
                    (handle.xkb_keymap_unref)(keymap);
                }
            }
            if let Some(context) = self.context.get_mut().unwrap().take() {
                if !context.is_null() {
                    (handle.xkb_context_unref)(context);
                }
            }
        }
    }
}

#[cfg(all(target_os = "linux", feature = "wayland"))]
impl Default for XkbKeymapManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            log::warn!("Failed to create XKB keymap manager: {}", e);
            Self {
                context: Mutex::new(None),
                keymap: Mutex::new(None),
                state: Mutex::new(None),
            }
        })
    }
}

#[cfg(not(all(target_os = "linux", feature = "wayland")))]
pub struct XkbKeymapManager;

#[cfg(not(all(target_os = "linux", feature = "wayland")))]
impl XkbKeymapManager {
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }

    pub fn update_keymap(&mut self, _keymap_string: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn update_modifiers(
        &mut self,
        _mods_depressed: u32,
        _mods_latched: u32,
        _mods_locked: u32,
        _group: u32,
    ) {
    }

    pub fn keycode_to_keysym(&self, _keycode: u32, _direction: xkb_key_direction) -> Option<u32> {
        None
    }

    pub fn keycode_to_utf8(&self, _keycode: u32, _direction: xkb_key_direction) -> Option<String> {
        None
    }

    pub fn keycode_to_keycode_name(&self, _keycode: u32) -> Option<String> {
        None
    }

    pub fn is_ready(&self) -> bool {
        false
    }
}

#[cfg(not(all(target_os = "linux", feature = "wayland")))]
impl Default for XkbKeymapManager {
    fn default() -> Self {
        Self
    }
}
