// SPDX-License-Identifier: LGPL-3.0-only

//! Diagnostic and profiling tools for performance analysis.
//!
//! This module provides performance metrics collection and analysis tools
//! to help identify bottlenecks and optimize layout/widget performance.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use taffy::NodeId;

/// Performance metrics for a single frame
#[derive(Debug, Clone, Default)]
pub struct FrameMetrics {
    /// Time spent in layout computation
    pub layout_time: Duration,
    /// Time spent in style computation
    pub style_time: Duration,
    /// Time spent in rendering
    pub render_time: Duration,
    /// Number of widgets processed
    pub widgets_processed: usize,
    /// Number of layout nodes created
    pub layout_nodes_created: usize,
    /// Number of style cache hits
    pub style_cache_hits: usize,
    /// Number of style cache misses
    pub style_cache_misses: usize,
    /// Number of nodes that were skipped due to early exit
    pub early_exit_count: usize,
}

/// Performance metrics aggregated over multiple frames
#[derive(Debug, Clone, Default)]
pub struct AggregatedMetrics {
    /// Total frames measured
    pub frame_count: usize,
    /// Average layout time per frame
    pub avg_layout_time: Duration,
    /// Average style time per frame
    pub avg_style_time: Duration,
    /// Average render time per frame
    pub avg_render_time: Duration,
    /// Average widgets processed per frame
    pub avg_widgets_processed: f64,
    /// Average layout nodes created per frame
    pub avg_layout_nodes_created: f64,
    /// Average style cache hit ratio (0.0 to 1.0)
    pub avg_style_cache_hit_ratio: f64,
    /// Average early exit ratio (0.0 to 1.0)
    pub avg_early_exit_ratio: f64,
}

/// Widget-level performance tracking
#[derive(Debug, Clone)]
pub struct WidgetMetrics {
    /// Widget ID
    pub widget_id: String,
    /// Number of times this widget caused layout invalidation
    pub invalidation_count: usize,
    /// Total time spent in this widget's layout_style() calls
    pub total_style_time: Duration,
    /// Number of style computations for this widget
    pub style_computation_count: usize,
    /// Average style computation time
    pub avg_style_time: Duration,
}

/// Performance diagnostics tracker
pub struct Diagnostics {
    /// Current frame metrics
    current_frame: FrameMetrics,
    /// Historical frame metrics (last N frames)
    frame_history: Vec<FrameMetrics>,
    /// Maximum number of frames to keep in history
    max_history_size: usize,
    /// Widget-level metrics
    widget_metrics: HashMap<String, WidgetMetrics>,
    /// Start time of current frame
    frame_start: Option<Instant>,
    /// Start time of current layout pass
    layout_start: Option<Instant>,
    /// Start time of current style computation
    style_start: Option<Instant>,
}

impl Diagnostics {
    /// Create a new diagnostics tracker
    pub fn new() -> Self {
        Self::with_history_size(100)
    }

    /// Create a new diagnostics tracker with specified history size
    pub fn with_history_size(max_history_size: usize) -> Self {
        Self {
            current_frame: FrameMetrics::default(),
            frame_history: Vec::with_capacity(max_history_size),
            max_history_size,
            widget_metrics: HashMap::new(),
            frame_start: None,
            layout_start: None,
            style_start: None,
        }
    }

    /// Start tracking a new frame
    pub fn start_frame(&mut self) {
        self.frame_start = Some(Instant::now());
        self.current_frame = FrameMetrics::default();
    }

    /// End tracking current frame and add to history
    pub fn end_frame(&mut self) {
        if let Some(start) = self.frame_start {
            // Frame time is already tracked separately
            self.frame_history.push(self.current_frame.clone());
            if self.frame_history.len() > self.max_history_size {
                self.frame_history.remove(0);
            }
        }
        self.frame_start = None;
    }

    /// Start tracking layout computation
    pub fn start_layout(&mut self) {
        self.layout_start = Some(Instant::now());
    }

    /// End tracking layout computation
    pub fn end_layout(&mut self) {
        if let Some(start) = self.layout_start {
            self.current_frame.layout_time += start.elapsed();
            self.layout_start = None;
        }
    }

    /// Start tracking style computation
    pub fn start_style(&mut self) {
        self.style_start = Some(Instant::now());
    }

    /// End tracking style computation
    pub fn end_style(&mut self) {
        if let Some(start) = self.style_start {
            self.current_frame.style_time += start.elapsed();
            self.style_start = None;
        }
    }

    /// Record widget processing
    pub fn record_widget_processed(&mut self) {
        self.current_frame.widgets_processed += 1;
    }

    /// Record layout node creation
    pub fn record_layout_node_created(&mut self) {
        self.current_frame.layout_nodes_created += 1;
    }

    /// Record style cache hit
    pub fn record_style_cache_hit(&mut self) {
        self.current_frame.style_cache_hits += 1;
    }

    /// Record style cache miss
    pub fn record_style_cache_miss(&mut self) {
        self.current_frame.style_cache_misses += 1;
    }

    /// Record early exit
    pub fn record_early_exit(&mut self) {
        self.current_frame.early_exit_count += 1;
    }

    /// Record widget invalidation
    pub fn record_widget_invalidation(&mut self, widget_id: &str) {
        let metrics = self.widget_metrics.entry(widget_id.to_string()).or_insert_with(|| {
            WidgetMetrics {
                widget_id: widget_id.to_string(),
                invalidation_count: 0,
                total_style_time: Duration::ZERO,
                style_computation_count: 0,
                avg_style_time: Duration::ZERO,
            }
        });
        metrics.invalidation_count += 1;
    }

    /// Record widget style computation time
    pub fn record_widget_style_time(&mut self, widget_id: &str, duration: Duration) {
        let metrics = self.widget_metrics.entry(widget_id.to_string()).or_insert_with(|| {
            WidgetMetrics {
                widget_id: widget_id.to_string(),
                invalidation_count: 0,
                total_style_time: Duration::ZERO,
                style_computation_count: 0,
                avg_style_time: Duration::ZERO,
            }
        });
        metrics.total_style_time += duration;
        metrics.style_computation_count += 1;
        metrics.avg_style_time = metrics.total_style_time / metrics.style_computation_count.max(1) as u32;
    }

    /// Get current frame metrics
    pub fn current_frame(&self) -> &FrameMetrics {
        &self.current_frame
    }

    /// Get aggregated metrics over recent frames
    pub fn aggregated_metrics(&self) -> AggregatedMetrics {
        if self.frame_history.is_empty() {
            return AggregatedMetrics::default();
        }

        let frame_count = self.frame_history.len();
        let mut total_layout_time = Duration::ZERO;
        let mut total_style_time = Duration::ZERO;
        let mut total_render_time = Duration::ZERO;
        let mut total_widgets = 0;
        let mut total_layout_nodes = 0;
        let mut total_cache_hits = 0;
        let mut total_cache_misses = 0;
        let mut total_early_exits = 0;
        let mut total_nodes = 0;

        for frame in &self.frame_history {
            total_layout_time += frame.layout_time;
            total_style_time += frame.style_time;
            total_render_time += frame.render_time;
            total_widgets += frame.widgets_processed;
            total_layout_nodes += frame.layout_nodes_created;
            total_cache_hits += frame.style_cache_hits;
            total_cache_misses += frame.style_cache_misses;
            total_early_exits += frame.early_exit_count;
            total_nodes += frame.layout_nodes_created;
        }

        let cache_total = total_cache_hits + total_cache_misses;
        let cache_hit_ratio = if cache_total > 0 {
            total_cache_hits as f64 / cache_total as f64
        } else {
            0.0
        };

        let early_exit_ratio = if total_nodes > 0 {
            total_early_exits as f64 / total_nodes as f64
        } else {
            0.0
        };

        AggregatedMetrics {
            frame_count,
            avg_layout_time: total_layout_time / frame_count as u32,
            avg_style_time: total_style_time / frame_count as u32,
            avg_render_time: total_render_time / frame_count as u32,
            avg_widgets_processed: total_widgets as f64 / frame_count as f64,
            avg_layout_nodes_created: total_layout_nodes as f64 / frame_count as f64,
            avg_style_cache_hit_ratio: cache_hit_ratio,
            avg_early_exit_ratio: early_exit_ratio,
        }
    }

    /// Get widget metrics
    pub fn widget_metrics(&self) -> &HashMap<String, WidgetMetrics> {
        &self.widget_metrics
    }

    /// Get top N widgets by invalidation count
    pub fn top_invalidating_widgets(&self, n: usize) -> Vec<&WidgetMetrics> {
        let mut widgets: Vec<&WidgetMetrics> = self.widget_metrics.values().collect();
        widgets.sort_by(|a, b| b.invalidation_count.cmp(&a.invalidation_count));
        widgets.into_iter().take(n).collect()
    }

    /// Clear all metrics
    pub fn clear(&mut self) {
        self.frame_history.clear();
        self.widget_metrics.clear();
        self.current_frame = FrameMetrics::default();
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}
