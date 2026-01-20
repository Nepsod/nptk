// SPDX-License-Identifier: LGPL-3.0-only

//! Object pools for temporary rendering data to reduce allocations.
//!
//! This module provides object pools for commonly allocated types in the rendering
//! pipeline, allowing reuse of buffers across frames to minimize allocations.

use std::collections::VecDeque;
use vello::kurbo::Rect;

/// A simple object pool for reusable Vec buffers.
pub struct VecPool<T> {
    /// Available buffers ready for reuse
    available: VecDeque<Vec<T>>,
    /// Maximum number of buffers to keep in the pool
    max_size: usize,
}

impl<T> VecPool<T> {
    /// Create a new pool with the specified maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            available: VecDeque::new(),
            max_size,
        }
    }

    /// Get a buffer from the pool, or create a new one if the pool is empty.
    pub fn get(&mut self) -> Vec<T> {
        self.available.pop_front().unwrap_or_else(Vec::new)
    }

    /// Get a buffer from the pool with at least the specified capacity.
    pub fn get_with_capacity(&mut self, capacity: usize) -> Vec<T> {
        if let Some(mut vec) = self.available.pop_front() {
            vec.clear();
            vec.reserve(capacity);
            vec
        } else {
            Vec::with_capacity(capacity)
        }
    }

    /// Return a buffer to the pool for reuse.
    /// The buffer will be cleared before being stored.
    pub fn return_buffer(&mut self, mut vec: Vec<T>) {
        vec.clear();
        if self.available.len() < self.max_size {
            self.available.push_back(vec);
        }
        // If pool is full, drop the buffer (let it be freed)
    }
}

impl<T> Default for VecPool<T> {
    fn default() -> Self {
        Self::new(4) // Default to 4 buffers
    }
}

/// A pool for Rect vectors (commonly used for dirty regions).
pub type RectPool = VecPool<Rect>;

/// A pool for temporary scene objects.
/// This is a placeholder - actual implementation depends on scene type.
pub struct ScenePool {
    /// Number of scenes available
    available_count: usize,
    /// Maximum scenes to keep
    max_size: usize,
}

impl ScenePool {
    /// Create a new scene pool.
    pub fn new(max_size: usize) -> Self {
        Self {
            available_count: 0,
            max_size,
        }
    }

    /// Check if a scene is available.
    pub fn has_available(&self) -> bool {
        self.available_count > 0
    }

    /// Mark a scene as returned to the pool.
    pub fn return_scene(&mut self) {
        if self.available_count < self.max_size {
            self.available_count += 1;
        }
    }

    /// Mark a scene as taken from the pool.
    pub fn take_scene(&mut self) {
        if self.available_count > 0 {
            self.available_count -= 1;
        }
    }
}

impl Default for ScenePool {
    fn default() -> Self {
        Self::new(2) // Default to 2 scenes
    }
}
