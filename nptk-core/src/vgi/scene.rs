//! Scene abstraction for vector graphics rendering.
//!
//! This module provides a unified scene interface that can work with different
//! rendering backends (Vello, Hybrid, and possibly future backends like tiny skia).

use std::any::Any;
use vello::Scene as VelloScene;
#[cfg(feature = "vello-hybrid")]
use vello_hybrid::Scene as HybridScene;

/// A trait for scene abstraction that allows different backends to provide
/// their own scene implementations.
///
/// This trait is object-safe to allow dynamic dispatch and scene composition.
pub trait SceneTrait: 'static {
    /// Reset the scene to its initial state.
    ///
    /// The scene should be equal to a newly created scene after this call.
    fn reset(&mut self);

    /// Reset only if needed based on dirty tracking.
    fn reset_if_needed(&mut self, dirty_tracker: &mut DirtyRegionTracker) {
        if dirty_tracker.full_reset_needed {
            self.reset();
            dirty_tracker.full_reset_needed = false;
        }
    }

    /// Get the width of the scene in pixels.
    fn width(&self) -> u32;

    /// Get the height of the scene in pixels.
    fn height(&self) -> u32;

    /// Returns this scene as an `Any` reference for type erasure.
    fn as_any(&self) -> &dyn Any;

    /// Returns this scene as a mutable `Any` reference for type erasure.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A unified scene that can be either Vello or Hybrid.
///
/// This enum wraps different scene types to provide a unified interface
/// for widget rendering, while still allowing backend-specific optimizations.
pub enum Scene {
    /// Standard Vello scene
    Vello(VelloScene),
    /// Vello Hybrid scene (CPU/GPU hybrid rendering)
    #[cfg(feature = "vello-hybrid")]
    Hybrid(HybridScene),
}

/// Element ID for tracking individual scene elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(u64);

impl ElementId {
    /// Create a new element ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the numeric ID
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// Tracks which regions of the scene need to be redrawn
#[derive(Default)]
pub struct DirtyRegionTracker {
    /// Whether the entire scene needs to be reset
    full_reset_needed: bool,
    /// Specific regions that need updates
    dirty_regions: Vec<vello::kurbo::Rect>,
    /// Whether any regions are dirty
    has_dirty_regions: bool,
    /// Map from element ID to scene layer/region for element-level tracking
    element_regions: std::collections::HashMap<ElementId, vello::kurbo::Rect>,
    /// Set of dirty element IDs for incremental updates
    dirty_elements: std::collections::HashSet<ElementId>,
}

impl DirtyRegionTracker {
    /// Create a new dirty region tracker
    pub fn new() -> Self {
        Self {
            full_reset_needed: true, // Start with full reset needed
            dirty_regions: Vec::new(),
            has_dirty_regions: false,
            element_regions: std::collections::HashMap::new(),
            dirty_elements: std::collections::HashSet::new(),
        }
    }

    /// Mark that a full scene reset is needed
    pub fn mark_full_reset_needed(&mut self) {
        self.full_reset_needed = true;
        self.has_dirty_regions = false;
        self.dirty_regions.clear();
        self.dirty_elements.clear();
    }

    /// Register an element with its region for tracking
    pub fn register_element(&mut self, element_id: ElementId, region: vello::kurbo::Rect) {
        self.element_regions.insert(element_id, region);
    }

    /// Mark an element as dirty
    pub fn mark_element_dirty(&mut self, element_id: ElementId) {
        if !self.full_reset_needed {
            self.dirty_elements.insert(element_id);
            // Add the element's region to dirty regions if known
            if let Some(region) = self.element_regions.get(&element_id) {
                self.add_dirty_region(*region);
            }
        }
    }

    /// Get dirty element IDs
    pub fn dirty_elements(&self) -> &std::collections::HashSet<ElementId> {
        &self.dirty_elements
    }

    /// Get element region if registered
    pub fn get_element_region(&self, element_id: ElementId) -> Option<&vello::kurbo::Rect> {
        self.element_regions.get(&element_id)
    }

    /// Check if a full reset is needed
    pub fn is_full_reset_needed(&self) -> bool {
        self.full_reset_needed
    }

    /// Add a dirty region that needs to be redrawn
    pub fn add_dirty_region(&mut self, rect: vello::kurbo::Rect) {
        if !self.full_reset_needed {
            self.dirty_regions.push(rect);
            self.has_dirty_regions = true;
        }
    }

    /// Check if there are any dirty regions
    pub fn has_dirty_regions(&self) -> bool {
        self.has_dirty_regions || self.full_reset_needed
    }

    /// Get all dirty regions
    pub fn get_dirty_regions(&self) -> &[vello::kurbo::Rect] {
        &self.dirty_regions
    }

    /// Calculate the total area of dirty regions
    pub fn dirty_regions_area(&self) -> f64 {
        if self.full_reset_needed {
            return f64::INFINITY;
        }
        self.dirty_regions
            .iter()
            .map(|r| r.width() * r.height())
            .sum()
    }

    /// Check if a rectangle intersects any dirty region
    pub fn intersects_dirty_region(&self, rect: vello::kurbo::Rect) -> bool {
        if self.full_reset_needed {
            return true;
        }
        self.dirty_regions
            .iter()
            .any(|dirty| {
                let intersection = dirty.intersect(rect);
                intersection.width() > 0.0 && intersection.height() > 0.0
            })
    }

    /// Check if dirty regions cover more than a threshold percentage of screen area
    pub fn should_use_full_reset(&self, screen_width: f64, screen_height: f64, threshold: f64) -> bool {
        if self.full_reset_needed {
            return true;
        }
        let screen_area = screen_width * screen_height;
        if screen_area == 0.0 {
            return true;
        }
        let dirty_area = self.dirty_regions_area();
        (dirty_area / screen_area) >= threshold
    }

    /// Clear all dirty state
    pub fn clear(&mut self) {
        self.full_reset_needed = false;
        self.has_dirty_regions = false;
        self.dirty_regions.clear();
        self.dirty_elements.clear();
    }

    /// Clear element tracking (call when structure changes significantly)
    pub fn clear_elements(&mut self) {
        self.element_regions.clear();
        self.dirty_elements.clear();
    }
}

impl Scene {
    /// Create a new unified scene based on the backend type.
    ///
    /// # Arguments
    /// * `backend` - The backend to use for scene creation
    /// * `width` - Scene width in pixels (used for Hybrid backend)
    /// * `height` - Scene height in pixels (used for Hybrid backend)
    ///
    /// # Note
    /// If Hybrid backend is requested but unavailable (due to wgpu version conflict),
    /// this will fall back to creating a Vello scene to match the renderer fallback behavior.
    pub fn new(backend: super::backend::Backend, _width: u32, _height: u32) -> Self {
        match backend {
            super::backend::Backend::Vello => Scene::Vello(VelloScene::new()),
            super::backend::Backend::Hybrid => {
                // CRITICAL: vello_hybrid uses wgpu 26.0.1, while vello uses wgpu 23.0.1.
                // These are incompatible versions. Since Renderer::new() falls back to Vello,
                // we must also fall back to Vello scene to avoid renderer/scene mismatch.
                log::warn!("Hybrid scene requested but unavailable, falling back to Vello scene");
                Scene::Vello(VelloScene::new())
            },
            super::backend::Backend::Custom(_) => {
                // For now, custom backends fall back to Vello
                // In the future, this can be extended with a registry or factory
                Scene::Vello(VelloScene::new())
            },
        }
    }

    /// Get a mutable reference to the Vello scene if this is a Vello scene.
    pub fn as_vello_mut(&mut self) -> Option<&mut VelloScene> {
        match self {
            Scene::Vello(scene) => Some(scene),
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(_) => None,
        }
    }

    /// Get a mutable reference to the Hybrid scene if this is a Hybrid scene.
    #[cfg(feature = "vello-hybrid")]
    pub fn as_hybrid_mut(&mut self) -> Option<&mut HybridScene> {
        match self {
            Scene::Vello(_) => None,
            Scene::Hybrid(scene) => Some(scene),
        }
    }

    /// Reset the scene to its initial state.
    ///
    /// This is a convenience method that calls the `SceneTrait::reset` method.
    pub fn reset(&mut self) {
        SceneTrait::reset(self);
    }

    /// Get the width of the scene in pixels.
    ///
    /// This is a convenience method that calls the `SceneTrait::width` method.
    pub fn width(&self) -> u32 {
        SceneTrait::width(self)
    }

    /// Get the height of the scene in pixels.
    ///
    /// This is a convenience method that calls the `SceneTrait::height` method.
    pub fn height(&self) -> u32 {
        SceneTrait::height(self)
    }
}


impl SceneTrait for Scene {
    fn reset(&mut self) {
        match self {
            Scene::Vello(scene) => scene.reset(),
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(scene) => scene.reset(),
        }
    }

    fn width(&self) -> u32 {
        match self {
            Scene::Vello(_) => 0, // Vello scenes don't track dimensions
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(scene) => scene.width() as u32,
        }
    }

    fn height(&self) -> u32 {
        match self {
            Scene::Vello(_) => 0, // Vello scenes don't track dimensions
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(scene) => scene.height() as u32,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
