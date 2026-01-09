//! Async icon loading logic for file icon widget.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use npio::service::icon::{CachedIcon, IconRegistry};
use npio::get_file_for_uri;
use nptk_core::app::context::AppContext;
use nptk_core::app::update::Update;
use nptk_services::filesystem::entry::FileEntry;
use nptk_services::thumbnail::npio_adapter::file_entry_to_uri;

/// Request icon loading asynchronously if not already cached.
///
/// # Arguments
///
/// * `icon_cache` - Shared icon cache
/// * `icon_registry` - Icon registry for loading icons
/// * `entry` - File entry to load icon for
/// * `size` - Icon size in pixels
/// * `context` - App context for spawning tasks
pub fn request_icon_loading(
    icon_cache: Arc<Mutex<std::collections::HashMap<(PathBuf, u32), Option<CachedIcon>>>>,
    icon_registry: Arc<IconRegistry>,
    entry: FileEntry,
    size: u32,
    context: AppContext,
) {
    let path = entry.path.clone();
    let cache_key = (path.clone(), size);
    
    // Check if already cached
    let needs_loading = {
        let cache = icon_cache.lock().unwrap();
        !cache.contains_key(&cache_key)
    };

    if !needs_loading {
        return;
    }

    let cache_clone = icon_cache.clone();
    let registry_clone = icon_registry.clone();
    let entry_clone = entry;
    let cache_key_clone = cache_key.clone();

    context.spawn_with_update(async move {
        let uri = file_entry_to_uri(&entry_clone);
        if let Ok(file) = get_file_for_uri(&uri) {
            let icon = registry_clone.get_file_icon(&*file, size).await;
            let mut cache = cache_clone.lock().unwrap();
            cache.insert(cache_key_clone, icon);
            
            // Return DRAW update to redraw the widget with the loaded icon
            Update::DRAW
        } else {
            Update::empty()
        }
    });
}
