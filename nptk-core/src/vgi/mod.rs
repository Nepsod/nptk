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

use vello::kurbo::{Affine, BezPath, Shape, Stroke};
use vello::peniko::{Brush, Fill};

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

pub use backend::Backend;
pub use gpu_context::{DeviceHandle, GpuContext};
pub use options::RendererOptions;
pub use platform::Platform;
pub use renderer::{Renderer, RendererTrait};
pub use scene::{Scene, SceneTrait};
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
        fill_rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    );

    /// Stroke a shape with the given brush.
    fn stroke(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Option<Affine>,
        shape: &BezPath,
    );

    /// Append another graphics scene to this one.
    ///
    /// Note: This method takes a Scene reference directly, since append
    /// operations typically work with Vello scenes. This maintains compatibility
    /// with existing widget code.
    fn append(&mut self, other: &vello::Scene, transform: Option<Affine>);

    /// Push a new layer with the given blend mode and transform.
    fn push_layer(
        &mut self,
        mix: vello::peniko::Mix,
        alpha: f32,
        transform: Affine,
        shape: &BezPath,
    );

    /// Pop the most recent layer.
    fn pop_layer(&mut self);

    /// Access the underlying Scene for operations that require it (e.g., Parley text rendering).
    /// Returns None if the graphics backend doesn't provide Scene access.
    fn as_scene_mut(&mut self) -> Option<&mut vello::Scene>;
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
#[cfg(feature = "vello-hybrid")]
pub fn graphics_from_scene(scene: &mut Scene) -> Option<Box<dyn Graphics + '_>> {
    match scene {
        Scene::Vello(vello_scene) => Some(Box::new(vello_vg::VelloGraphics::new(vello_scene))),
        Scene::Hybrid(hybrid_scene) => Some(Box::new(hybrid_vg::HybridGraphics::new(hybrid_scene))),
    }
}

#[cfg(not(feature = "vello-hybrid"))]
pub fn graphics_from_scene(scene: &mut Scene) -> Option<Box<dyn Graphics + '_>> {
    match scene {
        Scene::Vello(vello_scene) => Some(Box::new(vello_vg::VelloGraphics::new(vello_scene))),
    }
}

/// A default graphics implementation using Vello.
pub mod vello_vg;

/// A Hybrid graphics implementation using Vello Hybrid.
#[cfg(feature = "vello-hybrid")]
pub mod hybrid_vg;
