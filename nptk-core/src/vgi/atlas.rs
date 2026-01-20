// SPDX-License-Identifier: LGPL-3.0-only

//! Texture atlas management for icons and glyphs.
//!
//! This module provides infrastructure for packing icons and commonly used glyphs
//! into a single texture to reduce texture binding overhead during rendering.

use std::collections::HashMap;
use vello::kurbo::Rect;

/// Coordinates within the atlas for a single texture entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasCoordinates {
    /// X position in the atlas (normalized 0.0-1.0)
    pub u0: f32,
    /// Y position in the atlas (normalized 0.0-1.0)
    pub v0: f32,
    /// Width in the atlas (normalized 0.0-1.0)
    pub u1: f32,
    /// Height in the atlas (normalized 0.0-1.0)
    pub v1: f32,
}

/// An entry in the texture atlas.
#[derive(Debug, Clone)]
struct AtlasEntry {
    /// Coordinates within the atlas
    coords: AtlasCoordinates,
    /// Width of the texture entry
    width: u32,
    /// Height of the texture entry
    height: u32,
}

/// Manages a texture atlas for icons and glyphs.
/// 
/// This is a placeholder implementation that provides the infrastructure
/// for future texture atlas functionality. Full implementation would include:
/// - Texture packing algorithms (bin packing, etc.)
/// - GPU texture upload
/// - Lazy loading and upload
/// - Cache invalidation
pub struct TextureAtlas {
    /// Current atlas entries indexed by content hash or ID
    entries: HashMap<u64, AtlasEntry>,
    /// Current atlas texture width
    width: u32,
    /// Current atlas texture height
    height: u32,
    /// Next available position for packing (simplified - full impl would use bin packing)
    next_x: u32,
    next_y: u32,
    /// Current row height (for simplified packing)
    current_row_height: u32,
}

impl TextureAtlas {
    /// Create a new texture atlas with the specified dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            entries: HashMap::new(),
            width,
            height,
            next_x: 0,
            next_y: 0,
            current_row_height: 0,
        }
    }

    /// Add a texture to the atlas.
    /// 
    /// Returns the atlas coordinates if successfully added.
    /// Returns None if the atlas is full or the texture is too large.
    pub fn add_texture(&mut self, id: u64, texture_width: u32, texture_height: u32) -> Option<AtlasCoordinates> {
        // Check if already in atlas
        if let Some(entry) = self.entries.get(&id) {
            return Some(entry.coords);
        }

        // Simple packing: place textures row by row
        // Full implementation would use bin packing algorithm
        if texture_width > self.width || texture_height > self.height {
            return None; // Texture too large for atlas
        }

        // Check if we need to move to next row
        if self.next_x + texture_width > self.width {
            self.next_y += self.current_row_height;
            self.next_x = 0;
            self.current_row_height = 0;
        }

        // Check if we've run out of space
        if self.next_y + texture_height > self.height {
            return None; // Atlas is full
        }

        // Calculate normalized coordinates
        let coords = AtlasCoordinates {
            u0: self.next_x as f32 / self.width as f32,
            v0: self.next_y as f32 / self.height as f32,
            u1: (self.next_x + texture_width) as f32 / self.width as f32,
            v1: (self.next_y + texture_height) as f32 / self.height as f32,
        };

        // Store entry
        self.entries.insert(id, AtlasEntry {
            coords,
            width: texture_width,
            height: texture_height,
        });

        // Update next position
        self.next_x += texture_width;
        self.current_row_height = self.current_row_height.max(texture_height);

        Some(coords)
    }

    /// Get atlas coordinates for a texture by ID.
    pub fn get_coordinates(&self, id: u64) -> Option<AtlasCoordinates> {
        self.entries.get(&id).map(|entry| entry.coords)
    }

    /// Check if a texture is in the atlas.
    pub fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    /// Clear all entries from the atlas.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_x = 0;
        self.next_y = 0;
        self.current_row_height = 0;
    }

    /// Get the atlas dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the number of entries in the atlas.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for TextureAtlas {
    fn default() -> Self {
        Self::new(2048, 2048) // Default 2K x 2K atlas
    }
}
