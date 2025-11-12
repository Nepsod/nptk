//! Scene abstraction for vector graphics rendering.
//!
//! This module provides a unified scene interface that can work with different
//! rendering backends (Vello, Hybrid, and possibly future backends like tiny skia).

use std::any::Any;
#[cfg(feature = "vello")]
#[cfg(feature = "vello")]
use vello::Scene as VelloScene;
#[cfg(not(feature = "vello"))]
use crate::vg::Scene as VelloScene;
#[cfg(feature = "vello-hybrid")]
use vello_hybrid::Scene as HybridScene;
#[cfg(not(feature = "vello"))]
use crate::vg::Scene as PlaceholderScene;

/// A trait for scene abstraction that allows different backends to provide
/// their own scene implementations.
///
/// This trait is object-safe to allow dynamic dispatch and scene composition.
pub trait SceneTrait: 'static {
    /// Reset the scene to its initial state.
    ///
    /// The scene should be equal to a newly created scene after this call.
    fn reset(&mut self);

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
    #[cfg(feature = "vello")]
    Vello(VelloScene),
    /// Vello Hybrid scene (CPU/GPU hybrid rendering)
    #[cfg(feature = "vello-hybrid")]
    Hybrid(HybridScene),
    /// Placeholder scene used when Vello is unavailable
    #[cfg(not(feature = "vello"))]
    Placeholder(PlaceholderScene),
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
            #[cfg(feature = "vello")]
            super::backend::Backend::Vello => Scene::Vello(VelloScene::new()),
            #[cfg(not(feature = "vello"))]
            super::backend::Backend::Vello => Scene::Placeholder(PlaceholderScene::new()),
            super::backend::Backend::Hybrid => {
                #[cfg(feature = "vello-hybrid")]
                {
                    // CRITICAL: vello_hybrid uses wgpu 26.0.1, while vello uses wgpu 23.0.1.
                    // These are incompatible versions. Since Renderer::new() falls back to Vello,
                    // we must also fall back to Vello scene to avoid renderer/scene mismatch.
                    #[cfg(feature = "vello")]
                    {
                        log::warn!("Hybrid scene requested but unavailable, falling back to Vello scene");
                        Scene::Vello(VelloScene::new())
                    }
                    #[cfg(not(feature = "vello"))]
                    {
                        log::warn!("Hybrid scene requested but 'vello' feature is disabled; using placeholder scene");
                        Scene::Placeholder(PlaceholderScene::new())
                    }
                }
                #[cfg(not(feature = "vello-hybrid"))]
                {
                    #[cfg(feature = "vello")]
                    {
                        log::warn!("Hybrid scene requested but unavailable, falling back to Vello scene");
                        Scene::Vello(VelloScene::new())
                    }
                    #[cfg(not(feature = "vello"))]
                    {
                        log::warn!("Hybrid scene requested but 'vello' feature is disabled; using placeholder scene");
                        Scene::Placeholder(PlaceholderScene::new())
                    }
                }
            },
            super::backend::Backend::Custom(_) => {
                #[cfg(feature = "vello")]
                {
                    // For now, custom backends fall back to Vello
                    // In the future, this can be extended with a registry or factory
                    Scene::Vello(VelloScene::new())
                }
                #[cfg(not(feature = "vello"))]
                {
                    log::warn!("Custom backend requested but 'vello' feature is disabled; using placeholder scene");
                    Scene::Placeholder(PlaceholderScene::new())
                }
            },
        }
    }

    /// Get a mutable reference to the Vello scene if this is a Vello scene.
    #[cfg(feature = "vello")]
    pub fn as_vello_mut(&mut self) -> Option<&mut VelloScene> {
        match self {
            Scene::Vello(scene) => Some(scene),
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(_) => None,
            #[cfg(not(feature = "vello"))]
            Scene::Placeholder(_) => None,
        }
    }

    /// Get a mutable reference to the Hybrid scene if this is a Hybrid scene.
    #[cfg(feature = "vello-hybrid")]
    pub fn as_hybrid_mut(&mut self) -> Option<&mut HybridScene> {
        match self {
            Scene::Vello(_) => None,
            Scene::Hybrid(scene) => Some(scene),
            #[cfg(not(feature = "vello"))]
            Scene::Placeholder(_) => None,
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
            #[cfg(feature = "vello")]
            Scene::Vello(scene) => scene.reset(),
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(scene) => scene.reset(),
            #[cfg(not(feature = "vello"))]
            Scene::Placeholder(scene) => scene.reset(),
        }
    }

    fn width(&self) -> u32 {
        match self {
            #[cfg(feature = "vello")]
            Scene::Vello(_) => 0, // Vello scenes don't track dimensions
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(scene) => scene.width() as u32,
            #[cfg(not(feature = "vello"))]
            Scene::Placeholder(_scene) => 0,
        }
    }

    fn height(&self) -> u32 {
        match self {
            #[cfg(feature = "vello")]
            Scene::Vello(_) => 0, // Vello scenes don't track dimensions
            #[cfg(feature = "vello-hybrid")]
            Scene::Hybrid(scene) => scene.height() as u32,
            #[cfg(not(feature = "vello"))]
            Scene::Placeholder(_scene) => 0,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
