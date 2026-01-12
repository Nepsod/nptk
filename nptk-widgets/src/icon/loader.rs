//! Icon loading logic for icon widget.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use npio::service::icon::{CachedIcon, IconRegistry};
use nptk_core::app::context::AppContext;

/// Request icon loading if not already cached.
///
/// For theme icons, IconRegistry.get_icon() is synchronous, so we load immediately.
/// The result is stored in the cache.
///
/// # Arguments
///
/// * `icon_cache` - Shared icon cache
/// * `icon_registry` - Icon registry for loading icons
/// * `icon_name` - Icon name to load
/// * `size` - Icon size in pixels
/// * `context` - App context (for future async support)
pub fn request_icon_loading(
    icon_cache: Arc<Mutex<HashMap<(String, u32), Option<CachedIcon>>>>,
    icon_registry: Arc<IconRegistry>,
    icon_name: String,
    size: u32,
    _context: AppContext,
) {
    let cache_key = (icon_name.clone(), size);
    
    // Check if already cached
    let needs_loading = {
        let cache = icon_cache.lock().unwrap();
        !cache.contains_key(&cache_key)
    };

    if !needs_loading {
        return;
    }

    // For theme icons, get_icon() is synchronous
    let cached_icon = icon_registry.get_icon(&icon_name, size);
    
    // Store result in cache (Some(icon) if found, None if not found)
    let mut cache = icon_cache.lock().unwrap();
    cache.insert(cache_key, cached_icon);
}
