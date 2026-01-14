// SPDX-License-Identifier: LGPL-3.0-only
pub mod filesystem;
pub mod places;
pub mod bookmarks;
pub mod settings;
pub mod thumbnail;

// Re-export commonly used types from places and bookmarks
pub use places::{UserDirectory, get_home_file, get_user_special_file, get_user_special_dir_path, get_home_icon_name, get_directory_icon_name, reload_user_special_dirs_cache};
pub use bookmarks::{BookmarksService, Bookmark};
