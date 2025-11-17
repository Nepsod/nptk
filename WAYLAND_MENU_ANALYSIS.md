# Wayland Global Menu Implementation Analysis

## Current Architecture

### Three Separate Wayland Connections

1. **winit's Connection** (when using Platform::Winit)
   - Created internally by winit
   - Used to create the window surface
   - Not directly accessible

2. **WaylandClient Connection** (`nptk-core/src/vgi/wl_client.rs`)
   - Created for native Wayland support
   - Has its own `appmenu_manager` binding
   - Only works with surfaces from this connection

3. **PlasmaMenuClient Connection** (`nptk-core/src/vgi/plasma_menu.rs`)
   - Created specifically for Plasma AppMenu protocol
   - Has its own `appmenu_manager` binding
   - Only works with surfaces from this connection

### The Core Problem

**Wayland surfaces are tied to specific connections.** You cannot use a surface from one connection with a proxy (like `appmenu_manager`) from another connection. This is a fundamental limitation of Wayland.

When using `winit`:
- Window surface is on winit's connection
- `PlasmaMenuClient` has its own connection
- Cannot use `set_appmenu_for_surface()` with winit's surface

## Current Implementation Flow

### Menu Registration (X11)
1. ✅ Register with `com.canonical.AppMenu.Registrar` using window ID
2. ✅ Set X11 window properties (`_GTK_UNIQUE_BUS_NAME`, `_GTK_MENUBAR_OBJECT_PATH`)
3. ✅ Works correctly

### Menu Registration (Wayland with winit)
1. ✅ Register with `com.canonical.AppMenu.Registrar` using dummy window ID (1)
2. ✅ Store menu info globally via `menu_info::set_menu_info()`
3. ✅ Initialize `PlasmaMenuClient` (creates separate connection)
4. ❌ Cannot set appmenu on winit's surface (different connections)
5. ❌ Window app_id may not be set correctly
6. ❌ Plasma cannot match window to menu

## How Plasma Discovers Menus on Wayland

Plasma uses multiple mechanisms:

1. **org.kde.kwin.appmenu_manager Protocol** (Primary)
   - Requires surface from the same connection as the manager
   - Works for native Wayland apps
   - Does NOT work with winit (different connections)

2. **App ID Matching** (Fallback)
   - Plasma matches window `app_id` to menu service name
   - Requires correct `app_id` on window
   - Requires menu registered with matching service name

3. **Window Properties** (Legacy)
   - Some compositors read window properties
   - Not directly accessible with winit

## Issues Identified

### Issue 1: App ID Not Set on Winit Window
- Native Wayland sets `app_id` to "nptk" (see `wayland_surface.rs:435`)
- Winit window may not have `app_id` set, or it may be set to something else
- Plasma needs matching `app_id` to discover menu

### Issue 2: Menu Service Name Mismatch
- Menu service name: `com.nptk.menubar.app_{pid}`
- Window app_id: Unknown (likely not set or different)
- Plasma cannot match them

### Issue 3: No Way to Set AppMenu on Winit Surface
- `try_set_appmenu_for_winit_window()` correctly identifies the limitation
- Returns error: "Cannot use winit surface with separate Wayland connection"
- We're relying on app_id matching, but app_id isn't set

### Issue 4: PlasmaMenuClient Not Used
- `PlasmaMenuClient` is initialized but never actually used
- It can only work with surfaces from its own connection
- For winit, it's essentially dead code

## Solutions

### Solution 1: Set App ID on Winit Window (CRITICAL)

We need to ensure the winit window has the correct `app_id` set. This is how Plasma will match the window to the menu.

```rust
// In handler.rs, after creating window
if let Some(window) = &self.window {
    // Set app_id to match menu service name pattern
    // Or use a consistent app_id that matches the menu registration
    window.set_app_id("nptk"); // or derive from menu service name
}
```

**Note**: Check if winit's `Window` has a `set_app_id()` method. If not, we may need to use platform-specific APIs.

### Solution 2: Use Consistent App ID Pattern

Instead of random service names, use a consistent pattern:
- App ID: `com.nptk.app` (or just `nptk`)
- Menu service: `com.nptk.menubar.app_{pid}` or `com.nptk.menubar`

Plasma can match `com.nptk.app` to `com.nptk.menubar.*` if the pattern is consistent.

### Solution 3: Register Menu with App ID

When registering with the registrar, we could try to use the app_id instead of a dummy window ID. However, the registrar API may not support this.

### Solution 4: Use winit's Wayland Connection (Advanced)

If possible, get access to winit's Wayland connection and use it for the AppMenu protocol. This would require:
- Accessing winit's internal connection (may not be possible)
- Using that connection for `PlasmaMenuClient`
- This is likely not feasible without modifying winit

### Solution 5: Use Native Wayland Instead of Winit

For proper Wayland support, use `Platform::Wayland` instead of `Platform::Winit`. This requires:
- Setting `NPTK_RENDERER=wayland` environment variable
- Using native Wayland surface creation
- This gives us full control over the Wayland connection

## Recommended Fixes

### Immediate Fix: Set App ID

1. Find how to set app_id on winit window
2. Set it to a consistent value (e.g., "nptk")
3. Ensure menu service name can be matched to this app_id

### Medium-term: Improve Menu Service Naming

1. Use consistent app_id: `com.nptk.app` or `nptk`
2. Use menu service pattern that Plasma can match: `com.nptk.menubar` or `com.nptk.menubar.app`
3. Document the matching pattern

### Long-term: Consider Native Wayland

1. For full Wayland support, recommend using native Wayland mode
2. Document the limitations of winit-based Wayland menu support
3. Provide clear migration path

## Testing Checklist

- [ ] Verify winit window has app_id set
- [ ] Verify app_id matches menu service name pattern
- [ ] Test menu discovery with consistent app_id
- [ ] Test with native Wayland mode (NPTK_RENDERER=wayland)
- [ ] Verify Plasma can match window to menu
- [ ] Check Plasma logs for menu discovery errors

## References

- KDE AppMenu Protocol: https://wayland.app/protocols/kde-appmenu
- Wayland Connection Isolation: Surfaces are tied to connections
- Plasma Menu Discovery: Uses app_id matching as fallback
- Winit Limitations: Cannot access internal Wayland connection

