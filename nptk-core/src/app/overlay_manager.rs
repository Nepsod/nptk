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
            layout_node,
            is_visible: true,
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
        _theme: &mut dyn Theme,
        _info: &mut AppInfo,
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
            if let Some(_layer) = self.get_layer_mut(id) {
                // TODO: Implement actual widget rendering here
                // For now, we'll just pop the layer
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
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}
