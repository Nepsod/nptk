// SPDX-License-Identifier: LGPL-3.0-only

use std::collections::{HashMap, HashSet};
use taffy::NodeId;
use bitflags::bitflags;

bitflags! {
    /// Granular dirty flags for tracking what changed in a node.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DirtyFlags: u8 {
        /// Geometry changed (size, position, bounds)
        const GEOMETRY = 0b0001;
        /// Style changed (layout properties, display, etc.)
        const STYLE = 0b0010;
        /// Children changed (added, removed, reordered)
        const CHILDREN = 0b0100;
        /// Content changed (text, images, etc.)
        const CONTENT = 0b1000;
        /// Everything changed (full rebuild needed)
        const ALL = Self::GEOMETRY.bits() | Self::STYLE.bits() | Self::CHILDREN.bits() | Self::CONTENT.bits();
    }
}

/// Tracks which widgets need layout updates (invalidation).
///
/// This allows the layout system to only recompute layout for widgets
/// that have actually changed, rather than rebuilding the entire tree.
#[derive(Debug, Clone, Default)]
pub struct InvalidationTracker {
    /// Set of Taffy node IDs that are marked as dirty and need layout recomputation.
    dirty_nodes: HashSet<NodeId>,
    /// Map from widget path to Taffy node ID for quick lookup.
    widget_to_node: HashMap<String, NodeId>,
    /// Granular dirty flags per node - tracks what specifically changed.
    node_dirty_flags: HashMap<NodeId, DirtyFlags>,
    /// Performance metrics for layout computation.
    metrics: InvalidationMetrics,
}

/// Performance metrics for layout invalidation.
#[derive(Debug, Clone, Default)]
pub struct InvalidationMetrics {
    /// Number of nodes that were invalidated in the last layout pass.
    pub nodes_invalidated: usize,
    /// Number of nodes that were actually recomputed.
    pub nodes_recomputed: usize,
    /// Time spent in layout computation (in milliseconds).
    pub layout_time_ms: f64,
}

impl InvalidationTracker {
    /// Create a new invalidation tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a node as dirty (needs layout recomputation).
    /// Uses ALL flags by default for backward compatibility.
    pub fn mark_dirty(&mut self, node_id: NodeId) {
        self.mark_dirty_with_flags(node_id, DirtyFlags::ALL);
    }

    /// Mark a node as dirty with specific flags indicating what changed.
    pub fn mark_dirty_with_flags(&mut self, node_id: NodeId, flags: DirtyFlags) {
        self.dirty_nodes.insert(node_id);
        // Merge with existing flags if any
        let existing = self.node_dirty_flags.entry(node_id).or_insert(DirtyFlags::empty());
        *existing |= flags;
    }

    /// Mark a node as clean (no longer needs recomputation).
    pub fn mark_clean(&mut self, node_id: NodeId) {
        self.dirty_nodes.remove(&node_id);
        self.node_dirty_flags.remove(&node_id);
    }

    /// Get dirty flags for a node.
    pub fn get_dirty_flags(&self, node_id: NodeId) -> DirtyFlags {
        self.node_dirty_flags.get(&node_id).copied().unwrap_or(DirtyFlags::empty())
    }

    /// Check if a node has specific dirty flags set.
    pub fn has_dirty_flags(&self, node_id: NodeId, flags: DirtyFlags) -> bool {
        self.get_dirty_flags(node_id).intersects(flags)
    }

    /// Check if a node is dirty.
    pub fn is_dirty(&self, node_id: NodeId) -> bool {
        self.dirty_nodes.contains(&node_id)
    }

    /// Mark all nodes as clean (after layout recomputation).
    pub fn clear_all(&mut self) {
        self.dirty_nodes.clear();
        self.node_dirty_flags.clear();
    }

    /// Batch mark multiple nodes as dirty with the same flags.
    pub fn batch_mark_dirty(&mut self, node_ids: &[NodeId], flags: DirtyFlags) {
        for &node_id in node_ids {
            self.mark_dirty_with_flags(node_id, flags);
        }
    }

    /// Get all dirty nodes.
    pub fn dirty_nodes(&self) -> &HashSet<NodeId> {
        &self.dirty_nodes
    }

    /// Register a widget path to node ID mapping.
    pub fn register_widget(&mut self, widget_path: String, node_id: NodeId) {
        self.widget_to_node.insert(widget_path, node_id);
    }

    /// Get node ID for a widget path.
    pub fn get_node_id(&self, widget_path: &str) -> Option<NodeId> {
        self.widget_to_node.get(widget_path).copied()
    }

    /// Mark a widget as dirty by its path.
    pub fn mark_widget_dirty(&mut self, widget_path: &str) {
        if let Some(node_id) = self.get_node_id(widget_path) {
            self.mark_dirty(node_id);
        }
    }

    /// Propagate dirty state up the tree.
    ///
    /// When a child is marked dirty, all its ancestors should also be marked dirty
    /// since their layout may depend on the child's size.
    /// Uses GEOMETRY flag by default since child size changes affect parent geometry.
    pub fn propagate_dirty_up(&mut self, node_id: NodeId, parent_map: &HashMap<NodeId, NodeId>) {
        let flags = self.get_dirty_flags(node_id);
        let mut current = Some(node_id);
        while let Some(node) = current {
            // Propagate with GEOMETRY flag since child changes affect parent layout
            self.mark_dirty_with_flags(node, flags | DirtyFlags::GEOMETRY);
            current = parent_map.get(&node).copied();
        }
    }

    /// Get performance metrics.
    pub fn metrics(&self) -> &InvalidationMetrics {
        &self.metrics
    }

    /// Get mutable reference to metrics.
    pub fn metrics_mut(&mut self) -> &mut InvalidationMetrics {
        &mut self.metrics
    }

    /// Reset metrics for a new measurement period.
    pub fn reset_metrics(&mut self) {
        self.metrics = InvalidationMetrics::default();
    }
}

impl InvalidationMetrics {
    /// Create new metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a node was invalidated.
    pub fn record_invalidation(&mut self) {
        self.nodes_invalidated += 1;
    }

    /// Record that a node was recomputed.
    pub fn record_recomputation(&mut self) {
        self.nodes_recomputed += 1;
    }

    /// Record layout computation time.
    pub fn record_layout_time(&mut self, time_ms: f64) {
        self.layout_time_ms = time_ms;
    }

    /// Get the efficiency ratio (recomputed / invalidated).
    ///
    /// A ratio close to 1.0 means we're efficiently recomputing only what's needed.
    /// A ratio much less than 1.0 means we're invalidating more than we recompute.
    pub fn efficiency_ratio(&self) -> f64 {
        if self.nodes_invalidated == 0 {
            return 1.0;
        }
        self.nodes_recomputed as f64 / self.nodes_invalidated as f64
    }
}
