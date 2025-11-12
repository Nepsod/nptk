//! Vector Graphics Interface abstraction.
//!
//! This module provides a complete abstraction layer for graphics backends,
//! allowing widgets to be decoupled from specific rendering implementations.
//!
//! ## Structure
//!
//! - **[Graphics]**: Trait for widget-level drawing operations (backward compatible)
//! - **[Scene]**: Unified scene abstraction for different backends
//! - **[Renderer]**: Unified renderer abstraction for different backends
//! - **[Backend]**: Backend selection and configuration
//! - **[RendererOptions]**: Configuration for creating renderers
//!
//! ## Usage
//!
//! Widgets use the [Graphics] trait for drawing. The renderer and scene management
//! is handled by the application framework, allowing widgets to remain backend-agnostic.

use crate::vg::kurbo::{Affine, BezPath, Shape, Stroke};
use crate::vg::peniko::{Brush, Fill, Mix};
#[cfg(feature = "vello-hybrid")]
use vello_hybrid::Renderer as HybridRenderer;

// Re-export unified abstractions
pub mod backend;
pub mod gpu_context;
pub mod options;
pub mod platform;
pub mod renderer;
pub mod scene;
pub mod surface;
#[cfg(all(target_os = "linux", feature = "wayland"))]
pub mod wayland_surface;
#[cfg(all(target_os = "linux", feature = "wayland"))]
pub(crate) mod wl_client;

pub use self::backend::Backend;
pub use gpu_context::{DeviceHandle, GpuContext};
pub use self::options::RendererOptions;
pub use platform::Platform;
#[cfg(feature = "vello")]
pub use vello::RenderParams;
#[cfg(not(feature = "vello"))]
pub use renderer::RenderParams;
pub use self::renderer::Renderer;
pub use renderer::RendererTrait;
pub use self::scene::{Scene, SceneTrait};
pub use surface::{Surface, SurfaceTrait};

/// A trait for rendering vector graphics.
///
/// This trait abstracts over different rendering backends, allowing widgets to
/// be written without being tied to a specific implementation.
///
/// Note: Methods use `&BezPath` for object-safety. To use concrete shape types
/// (Rect, RoundedRect, Line, etc.), convert them to BezPath using `shape.to_path(0.1)`.
pub trait Graphics {
    /// Fill a shape with the given brush.
    fn fill(
        &mut self,
        _fill_rule: Fill,
        _transform: Affine,
        _brush: &Brush,
        _brush_transform: Option<Affine>,
        _shape: &BezPath,
    ) {
    }

    /// Stroke a shape with the given brush.
    fn stroke(
        &mut self,
        _style: &Stroke,
        _transform: Affine,
        _brush: &Brush,
        _brush_transform: Option<Affine>,
        _shape: &BezPath,
    ) {
    }

    /// Append another graphics scene to this one.
    fn append(&mut self, _other: &crate::vg::Scene, _transform: Option<Affine>) {}

    /// Push a new layer with the given blend mode and transform.
    fn push_layer(
        &mut self,
        _mix: Mix,
        _alpha: f32,
        _transform: Affine,
        _shape: &BezPath,
    ) {
    }

    /// Pop the most recent layer.
    fn pop_layer(&mut self) {}

    /// Access the underlying Scene for operations that require it (e.g., Parley text rendering).
    /// Returns None if the graphics backend doesn't provide Scene access.
    fn as_scene_mut(&mut self) -> Option<&mut crate::vg::Scene> {
        None
    }
}

/// Helper function to convert a shape to BezPath for use with Graphics trait.
pub fn shape_to_path(shape: &impl Shape) -> BezPath {
    shape.to_path(0.1)
}

/// Create a Graphics implementation from a unified Scene.
///
/// This helper function allows widgets to draw to a unified Scene by creating
/// an appropriate Graphics implementation based on the scene's backend.
///
/// # Returns
/// * `Some(Box<dyn Graphics>)` if the scene backend is supported
/// * `None` if the scene backend doesn't have a Graphics implementation yet
#[cfg(feature = "vello")]
#[cfg(feature = "vello-hybrid")]
pub fn graphics_from_scene(scene: &mut Scene) -> Option<Box<dyn Graphics + '_>> {
    match scene {
        Scene::Vello(vello_scene) => Some(Box::new(vello_vg::VelloGraphics::new(vello_scene))),
        Scene::Hybrid(hybrid_scene) => Some(Box::new(hybrid_vg::HybridGraphics::new(hybrid_scene))),
    }
}

#[cfg(all(feature = "vello", not(feature = "vello-hybrid")))]
pub fn graphics_from_scene(scene: &mut Scene) -> Option<Box<dyn Graphics + '_>> {
    match scene {
        Scene::Vello(vello_scene) => Some(Box::new(vello_vg::VelloGraphics::new(vello_scene))),
    }
}

#[cfg(not(feature = "vello"))]
pub fn graphics_from_scene(scene: &mut Scene) -> Option<Box<dyn Graphics + '_>> {
    match scene {
        Scene::Placeholder(placeholder) => {
            Some(Box::new(vello_vg::VelloGraphics::new(placeholder)))
        },
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

/// A default graphics implementation using Vello.
#[cfg(feature = "vello")]
pub mod vello_vg;

#[cfg(not(feature = "vello"))]
pub mod vello_vg {
    use super::Graphics;
    use crate::vg::Scene;

    /// Placeholder graphics implementation used when Vello is disabled.
    pub struct VelloGraphics<'a> {
        scene: &'a mut Scene,
    }

    impl<'a> VelloGraphics<'a> {
        /// Create a new placeholder graphics instance.
        pub fn new(scene: &'a mut Scene) -> Self {
            Self { scene }
        }

        /// Access the underlying placeholder scene.
        pub fn scene(&mut self) -> &mut Scene {
            self.scene
        }
    }

    impl<'a> Graphics for VelloGraphics<'a> {}
}

/// A Hybrid graphics implementation using Vello Hybrid.
#[cfg(feature = "vello-hybrid")]
pub mod hybrid_vg;
