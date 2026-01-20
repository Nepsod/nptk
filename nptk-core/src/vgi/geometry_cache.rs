// SPDX-License-Identifier: LGPL-3.0-only

//! Geometry caching for shapes to avoid repeated tessellation.
//!
//! This module provides caching for generated geometry (meshes, paths) for shapes
//! like rectangles, rounded rectangles, borders, etc. When size and style are unchanged,
//! cached geometry can be reused instead of regenerating it.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use vello::kurbo::{RoundedRect, Rect};

/// Cache key for geometry based on shape properties
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeometryKey {
    /// Rectangle geometry
    Rectangle {
        width: u32,
        height: u32,
    },
    /// Rounded rectangle geometry
    RoundedRect {
        width: u32,
        height: u32,
        radius_bits: u32, // f32 converted to bits for hashing
    },
    /// Border geometry
    Border {
        width: u32,
        height: u32,
        border_width_bits: u32, // f32 converted to bits for hashing
        radius_bits: Option<u32>, // f32 converted to bits for hashing
    },
}

impl GeometryKey {
    /// Create a rounded rectangle key from f32 radius
    pub fn rounded_rect(width: u32, height: u32, radius: f32) -> Self {
        Self::RoundedRect {
            width,
            height,
            radius_bits: radius.to_bits(),
        }
    }

    /// Create a border key from f32 values
    pub fn border(width: u32, height: u32, border_width: f32, radius: Option<f32>) -> Self {
        Self::Border {
            width,
            height,
            border_width_bits: border_width.to_bits(),
            radius_bits: radius.map(|r| r.to_bits()),
        }
    }
}

/// Cached geometry data (placeholder - actual geometry type depends on rendering backend)
/// For now, this is a placeholder that can be extended with actual geometry types
#[derive(Clone)]
pub struct CachedGeometry {
    /// Version of this cached geometry
    pub version: u64,
    /// Bounds of the geometry
    pub bounds: Rect,
}

/// Geometry cache for reusing generated shape geometry
pub struct GeometryCache {
    /// Cache of geometry by key
    cache: HashMap<GeometryKey, CachedGeometry>,
    /// Current version counter for cache invalidation
    version: u64,
    /// Maximum cache size
    max_size: usize,
}

impl GeometryCache {
    /// Create a new geometry cache with default size
    pub fn new() -> Self {
        Self::with_capacity(500)
    }

    /// Create a new geometry cache with specified capacity
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            version: 0,
            max_size,
        }
    }

    /// Get cached geometry if available and version matches
    pub fn get(&self, key: &GeometryKey) -> Option<&CachedGeometry> {
        self.cache.get(key)
    }

    /// Insert geometry into cache
    pub fn insert(&mut self, key: GeometryKey, bounds: Rect) {
        // Evict old entries if cache is full
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove oldest entries (first N entries)
            // In a production system, this could use LRU eviction
            let to_remove = self.cache.len() - self.max_size + 1;
            let keys: Vec<_> = self.cache.keys().take(to_remove).cloned().collect();
            for key in keys {
                self.cache.remove(&key);
            }
        }

        self.cache.insert(
            key,
            CachedGeometry {
                version: self.version,
                bounds,
            },
        );
    }

    /// Invalidate all cached geometry (increment version)
    pub fn invalidate_all(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.cache.clear();
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for GeometryCache {
    fn default() -> Self {
        Self::new()
    }
}
