// SPDX-License-Identifier: MIT OR Apache-2.0

//! Overlay and popup management system
//! 
//! This module provides a layered overlay system inspired by both Xilem's scene layering
//! and GNUStep's window level system. It supports both simple scene-based overlays
//! and complex separate render contexts for modals and complex popups.

use std::collections::HashMap;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{BlendMode, Color};
use crate::widget::Widget;
use crate::layout::LayoutNode;
use nptk_theme::theme::Theme;
use crate::app::info::AppInfo;
use crate::app::context::AppContext;
use nalgebra::Vector2;

/// Unique identifier for overlays
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayId(pub u32);

impl OverlayId {
    /// Create a new overlay ID with the given value
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Overlay levels inspired by GNUStep's window levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OverlayLevel {
    /// Tooltips and simple hints (level 1)
    Tooltip = 1,
    /// Dropdowns, context menus (level 2) 
    Popup = 2,
    /// Floating panels (level 3)
    Floating = 3,
    /// Modal dialogs (level 100)
    Modal = 100,
}

/// Information about a scene-based overlay layer
pub struct OverlayLayer {
    /// Unique identifier for this overlay
    pub id: OverlayId,
    /// The z-index level of this overlay
    pub level: OverlayLevel,
    /// Blend mode for rendering this layer
    pub blend_mode: BlendMode,
    /// Opacity of this layer (0.0 to 1.0)
    pub opacity: f32,
    /// Transform matrix for this layer
    pub transform: Affine,
    /// Optional clipping rectangle for this layer
    pub clip_rect: Option<Rect>,
    /// The widget content to render in this layer
    pub content: Box<dyn Widget>,
    /// Layout information for the content
    pub layout_node: LayoutNode,
    /// Whether this layer is currently visible
    pub is_visible: bool,
    /// Whether clicking outside this overlay should close it
    pub click_outside_to_close: bool,
    /// Whether this overlay was just created (prevents immediate click-outside detection)
    pub just_created: bool,
    /// Frame count since creation (used to delay click-outside detection)
    pub creation_frame: u32,
    /// The desired position of this overlay
    pub desired_position: Vector2<f64>,
    /// The actual rendered position of this overlay (may differ from desired due to bounds checking)
    pub actual_position: Vector2<f64>,
    /// The size of this overlay
    pub size: Vector2<f64>,
    /// The anchor point this overlay is positioned relative to
    pub anchor_position: Option<Vector2<f64>>,
    /// The anchor bounds this overlay is positioned relative to
    pub anchor_bounds: Option<Rect>,
}

/// Information about a separate render context overlay
pub struct OverlayContext {
    /// Unique identifier for this overlay
    pub id: OverlayId,
    /// The z-index level of this overlay
    pub level: OverlayLevel,
    /// The scene for rendering this overlay
    pub scene: Scene,
    /// Whether this overlay is modal (blocks interaction with other content)
    pub is_modal: bool,
    /// Whether this overlay blocks events from reaching underlying content
    pub blocks_events: bool,
    /// Whether clicking outside this overlay should close it
    pub click_outside_to_close: bool,
    /// Optional backdrop color for modal overlays
    pub backdrop_color: Option<Color>,
    /// Whether this overlay is currently visible
    pub is_visible: bool,
}

/// Global overlay manager that handles all overlays in the application
pub struct OverlayManager {
    /// Scene-based overlay layers (for simple overlays like tooltips)
    layers: Vec<OverlayLayer>,
    /// Separate render context overlays (for complex overlays like modals)
    contexts: Vec<OverlayContext>,
    /// Next available overlay ID
    next_id: u32,
    /// Map of overlay IDs to their indices in the layers/contexts vectors
    id_to_index: HashMap<OverlayId, usize>,
}

impl OverlayManager {
    /// Create a new overlay manager
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            contexts: Vec::new(),
            next_id: 1,
            id_to_index: HashMap::new(),
        }
    }

    /// Generate a new unique overlay ID
    fn next_id(&mut self) -> OverlayId {
        let id = OverlayId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a new scene-based overlay layer
    pub fn add_layer(
        &mut self,
        level: OverlayLevel,
        content: Box<dyn Widget>,
        layout_node: LayoutNode,
    ) -> OverlayId {
        let id = self.next_id();
        let layer = OverlayLayer {
            id,
            level,
            blend_mode: BlendMode::default(),
            opacity: 1.0,
            transform: Affine::IDENTITY,
            clip_rect: None,
            content,
            layout_node: layout_node.clone(),
            is_visible: true,
            click_outside_to_close: true, // Default to true for most overlays
            just_created: true, // Prevent immediate click-outside detection
            creation_frame: 0, // Start at frame 0
            desired_position: Vector2::new(layout_node.layout.location.x as f64, layout_node.layout.location.y as f64),
            actual_position: Vector2::new(layout_node.layout.location.x as f64, layout_node.layout.location.y as f64),
            size: Vector2::new(layout_node.layout.size.width as f64, layout_node.layout.size.height as f64),
            anchor_position: None,
            anchor_bounds: None,
        };
        
        let index = self.layers.len();
        self.layers.push(layer);
        self.id_to_index.insert(id, index);
        
        id
    }

    /// Add a new separate render context overlay
    pub fn add_context(
        &mut self,
        level: OverlayLevel,
        is_modal: bool,
        blocks_events: bool,
        click_outside_to_close: bool,
        backdrop_color: Option<Color>,
    ) -> OverlayId {
        let id = self.next_id();
        let context = OverlayContext {
            id,
            level,
            scene: Scene::new(),
            is_modal,
            blocks_events,
            click_outside_to_close,
            backdrop_color,
            is_visible: true,
        };
        
        let index = self.contexts.len();
        self.contexts.push(context);
        self.id_to_index.insert(id, index);
        
        id
    }

    /// Remove an overlay by ID
    pub fn remove_overlay(&mut self, id: OverlayId) -> bool {
        if let Some(&index) = self.id_to_index.get(&id) {
            // Check if it's a layer or context
            if index < self.layers.len() {
                self.layers.remove(index);
                // Update indices for remaining layers
                for (_, stored_index) in self.id_to_index.iter_mut() {
                    if *stored_index > index {
                        *stored_index -= 1;
                    }
                }
            } else {
                let context_index = index - self.layers.len();
                self.contexts.remove(context_index);
                // Update indices for remaining contexts
                for (_, stored_index) in self.id_to_index.iter_mut() {
                    if *stored_index > index {
                        *stored_index -= 1;
                    }
                }
            }
            self.id_to_index.remove(&id);
            true
        } else {
            false
        }
    }

    /// Get a mutable reference to an overlay layer
    pub fn get_layer_mut(&mut self, id: OverlayId) -> Option<&mut OverlayLayer> {
        if let Some(&index) = self.id_to_index.get(&id) {
            if index < self.layers.len() {
                Some(&mut self.layers[index])
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get a mutable reference to an overlay context
    pub fn get_context_mut(&mut self, id: OverlayId) -> Option<&mut OverlayContext> {
        if let Some(&index) = self.id_to_index.get(&id) {
            if index >= self.layers.len() {
                let context_index = index - self.layers.len();
                Some(&mut self.contexts[context_index])
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Set overlay visibility
    pub fn set_visible(&mut self, id: OverlayId, visible: bool) -> bool {
        if let Some(layer) = self.get_layer_mut(id) {
            layer.is_visible = visible;
            true
        } else if let Some(context) = self.get_context_mut(id) {
            context.is_visible = visible;
            true
        } else {
            false
        }
    }

    /// Get all visible layers sorted by level (lowest first)
    pub fn get_visible_layers(&self) -> Vec<&OverlayLayer> {
        let mut visible_layers: Vec<&OverlayLayer> = self.layers
            .iter()
            .filter(|layer| layer.is_visible)
            .collect();
        
        visible_layers.sort_by_key(|layer| layer.level);
        visible_layers
    }

    /// Get all visible contexts sorted by level (lowest first)
    pub fn get_visible_contexts(&self) -> Vec<&OverlayContext> {
        let mut visible_contexts: Vec<&OverlayContext> = self.contexts
            .iter()
            .filter(|context| context.is_visible)
            .collect();
        
        visible_contexts.sort_by_key(|context| context.level);
        visible_contexts
    }

    /// Render all visible overlays to the main scene
    pub fn render_overlays(
        &mut self,
        main_scene: &mut Scene,
        theme: &mut dyn Theme,
        context: AppContext,
    ) {
        // Render scene-based layers
        // We need to collect the visible layers first to avoid borrow checker issues
        let visible_layers: Vec<(OverlayId, OverlayLevel, BlendMode, f32, Affine, Option<Rect>)> = self.layers
            .iter()
            .filter(|layer| layer.is_visible)
            .map(|layer| (
                layer.id,
                layer.level,
                layer.blend_mode,
                layer.opacity,
                layer.transform,
                layer.clip_rect,
            ))
            .collect();

        // Sort by level
        let mut sorted_layers = visible_layers;
        sorted_layers.sort_by_key(|(_, level, _, _, _, _)| *level);

        for (id, _level, blend_mode, opacity, transform, clip_rect) in sorted_layers {
            // Push layer with specified blend mode and opacity
            if let Some(clip_rect) = clip_rect {
                main_scene.push_layer(
                    blend_mode,
                    opacity,
                    transform,
                    &clip_rect,
                );
            } else {
                main_scene.push_layer(
                    blend_mode,
                    opacity,
                    transform,
                    &Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY),
                );
            }

            // Render the overlay content
            if let Some(layer) = self.get_layer_mut(id) {
                // Create a temporary AppInfo for rendering
                let mut temp_info = AppInfo::default();
                
                // Render the widget content using the provided context
                layer.content.render(
                    main_scene,
                    theme,
                    &layer.layout_node,
                    &mut temp_info,
                    context.clone(),
                );
            }
            
            // Pop the layer
            main_scene.pop_layer();
        }
    }

    /// Check if any modal overlays are currently visible
    pub fn has_modal_overlay(&self) -> bool {
        self.contexts.iter().any(|context| context.is_visible && context.is_modal)
    }

    /// Get the highest visible overlay level
    pub fn get_highest_level(&self) -> Option<OverlayLevel> {
        let mut highest = None;
        
        for layer in &self.layers {
            if layer.is_visible && (highest.is_none() || layer.level > highest.unwrap()) {
                highest = Some(layer.level);
            }
        }
        
        for context in &self.contexts {
            if context.is_visible && (highest.is_none() || context.level > highest.unwrap()) {
                highest = Some(context.level);
            }
        }
        
        highest
    }

    /// Handle mouse click events for click-outside detection
    pub fn handle_mouse_click(&mut self, x: f64, y: f64) -> bool {
        // If there's a modal overlay, don't process click-outside detection
        // Modals should handle their own click events
        if self.has_modal_overlay() {
            return true; // Modal is handling the event
        }
        
        let mut overlays_to_close = Vec::new();
        
        // Check all visible overlays for click-outside detection
        for layer in &self.layers {
            if layer.is_visible && layer.click_outside_to_close {
                if layer.just_created {
                    // Skip click-outside detection for newly created overlays
                } else {
                    let bounds = Rect::new(
                        layer.actual_position.x,
                        layer.actual_position.y,
                        layer.actual_position.x + layer.size.x,
                        layer.actual_position.y + layer.size.y,
                    );
                    
                    // Check if click is outside both the overlay and its anchor bounds
                    let mut click_outside = !bounds.contains((x, y));
                    
                    // If there are anchor bounds, also check if click is outside those
                    if let Some(anchor_bounds) = layer.anchor_bounds {
                        println!("OverlayManager: Checking click ({}, {}) against anchor bounds {:?}", x, y, anchor_bounds);
                        if anchor_bounds.contains((x, y)) {
                            println!("OverlayManager: Click is on anchor, keeping overlay open");
                            click_outside = false; // Click is on the anchor, don't close
                        }
                    }
                    
                    // If click is outside both overlay and anchor, mark it for closing
                    if click_outside {
                        println!("OverlayManager: Click outside both overlay and anchor, closing overlay {:?}", layer.id);
                        overlays_to_close.push(layer.id);
                    } else {
                        println!("OverlayManager: Click inside overlay or anchor, keeping open");
                    }
                }
            }
        }
        
        for context in &self.contexts {
            if context.is_visible && context.click_outside_to_close {
                // TODO: Implement bounds checking for contexts
                // For now, assume all context overlays should close on outside click
                overlays_to_close.push(context.id);
            }
        }
        
        // Close overlays that should close on outside click
        let mut any_closed = false;
        for overlay_id in overlays_to_close {
            if self.remove_overlay(overlay_id) {
                any_closed = true;
            }
        }
        
        any_closed
    }

    /// Check if a point is inside any visible overlay
    pub fn is_point_inside_overlay(&self, x: f64, y: f64) -> bool {
        // Check all visible layers
        for layer in &self.layers {
            if layer.is_visible {
                let bounds = Rect::new(
                    layer.actual_position.x,
                    layer.actual_position.y,
                    layer.actual_position.x + layer.size.x,
                    layer.actual_position.y + layer.size.y,
                );
                if bounds.contains((x, y)) {
                    return true;
                }
            }
        }
        
        // TODO: Check contexts when we have bounds for them
        false
    }

    /// Set click-outside-to-close behavior for a layer
    pub fn set_layer_click_outside_to_close(&mut self, id: OverlayId, enabled: bool) -> bool {
        if let Some(layer) = self.get_layer_mut(id) {
            layer.click_outside_to_close = enabled;
            true
        } else {
            false
        }
    }

    /// Set click-outside-to-close behavior for a context
    pub fn set_context_click_outside_to_close(&mut self, id: OverlayId, enabled: bool) -> bool {
        if let Some(context) = self.get_context_mut(id) {
            context.click_outside_to_close = enabled;
            true
        } else {
            false
        }
    }

    /// Clear the just_created flag for all overlays (call this after each frame)
    pub fn clear_just_created_flags(&mut self) {
        for layer in &mut self.layers {
            if layer.just_created {
                layer.creation_frame += 1;
                // Clear the flag after 2 frames to allow for proper event handling
                if layer.creation_frame >= 2 {
                    layer.just_created = false;
                }
            }
        }
    }

    /// Handle keyboard events for modal overlays
    pub fn handle_keyboard_event(&mut self, key_code: &str) -> bool {
        // If there's a modal overlay, handle ESC key to close it
        if self.has_modal_overlay() && key_code == "Escape" {
            // Close the highest level modal overlay
            let mut modal_to_close = None;
            let mut highest_level = None;
            
            for context in &self.contexts {
                if context.is_visible && context.is_modal {
                    if highest_level.is_none() || context.level > highest_level.unwrap() {
                        highest_level = Some(context.level);
                        modal_to_close = Some(context.id);
                    }
                }
            }
            
            if let Some(modal_id) = modal_to_close {
                return self.remove_overlay(modal_id);
            }
        }
        
        false
    }

    /// Check if events should be blocked due to modal overlays
    pub fn should_block_events(&self) -> bool {
        self.has_modal_overlay()
    }

    /// Get the topmost modal overlay ID
    pub fn get_top_modal(&self) -> Option<OverlayId> {
        let mut top_modal = None;
        let mut highest_level = None;
        
        for context in &self.contexts {
            if context.is_visible && context.is_modal {
                if highest_level.is_none() || context.level > highest_level.unwrap() {
                    highest_level = Some(context.level);
                    top_modal = Some(context.id);
                }
            }
        }
        
        top_modal
    }

    /// Set the position and anchor for a layer
    pub fn set_layer_position(
        &mut self,
        id: OverlayId,
        position: Vector2<f64>,
        anchor_bounds: Option<Rect>,
    ) -> bool {
        // Find the layer index
        let layer_index = if let Some((index, _)) = self.layers.iter().enumerate().find(|(_, layer)| layer.id == id) {
            index
        } else {
            return false;
        };
        
        // Update the layer directly
        let layer = &mut self.layers[layer_index];
        let size = layer.size;
        layer.desired_position = position;
        layer.anchor_bounds = anchor_bounds;
        layer.actual_position = Self::calculate_smart_position_static(
            position,
            size,
            anchor_bounds,
            Vector2::new(800.0, 600.0), // TODO: Get actual screen size
        );
        true
    }

    /// Set the size of a layer
    pub fn set_layer_size(&mut self, id: OverlayId, size: Vector2<f64>) -> bool {
        // Find the layer index
        let layer_index = if let Some((index, _)) = self.layers.iter().enumerate().find(|(_, layer)| layer.id == id) {
            index
        } else {
            return false;
        };
        
        // Update the layer directly
        let layer = &mut self.layers[layer_index];
        let desired_position = layer.desired_position;
        let anchor_bounds = layer.anchor_bounds;
        layer.size = size;
        // Recalculate position with new size
        layer.actual_position = Self::calculate_smart_position_static(
            desired_position,
            size,
            anchor_bounds,
            Vector2::new(800.0, 600.0), // TODO: Get actual screen size
        );
        true
    }

    /// Calculate smart position that keeps overlay within screen bounds (static version)
    fn calculate_smart_position_static(
        desired_position: Vector2<f64>,
        overlay_size: Vector2<f64>,
        anchor_bounds: Option<Rect>,
        screen_size: Vector2<f64>,
    ) -> Vector2<f64> {
        let mut position = desired_position;
        
        // Check horizontal bounds
        if position.x + overlay_size.x > screen_size.x {
            // Try to position to the left of the anchor
            if let Some(anchor) = anchor_bounds {
                position.x = anchor.min_x() - overlay_size.x - 10.0; // 10px gap
            } else {
                position.x = screen_size.x - overlay_size.x - 10.0;
            }
            
            // If still out of bounds, center horizontally
            if position.x < 0.0 {
                position.x = (screen_size.x - overlay_size.x) / 2.0;
            }
        }
        
        // Check vertical bounds
        if position.y + overlay_size.y > screen_size.y {
            // Try to position above the anchor
            if let Some(anchor) = anchor_bounds {
                position.y = anchor.min_y() - overlay_size.y - 10.0; // 10px gap
            } else {
                position.y = screen_size.y - overlay_size.y - 10.0;
            }
            
            // If still out of bounds, center vertically
            if position.y < 0.0 {
                position.y = (screen_size.y - overlay_size.y) / 2.0;
            }
        }
        
        // Ensure minimum margins from screen edges
        let margin = 10.0;
        position.x = position.x.max(margin);
        position.y = position.y.max(margin);
        
        position
    }

    /// Update overlay positions based on screen size changes
    pub fn update_positions_for_screen_size(&mut self, screen_size: Vector2<f64>) {
        for layer in &mut self.layers {
            if layer.is_visible {
                let desired_position = layer.desired_position;
                let size = layer.size;
                let anchor_bounds = layer.anchor_bounds;
                layer.actual_position = Self::calculate_smart_position_static(
                    desired_position,
                    size,
                    anchor_bounds,
                    screen_size,
                );
            }
        }
    }

    /// Get the bounds of a layer
    pub fn get_layer_bounds(&self, id: OverlayId) -> Option<Rect> {
        if let Some(layer) = self.layers.iter().find(|l| l.id == id) {
            Some(Rect::new(
                layer.actual_position.x,
                layer.actual_position.y,
                layer.actual_position.x + layer.size.x,
                layer.actual_position.y + layer.size.y,
            ))
        } else {
            None
        }
    }
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}
