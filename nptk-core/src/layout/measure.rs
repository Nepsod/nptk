// SPDX-License-Identifier: LGPL-3.0-only
use nalgebra::Vector2;
use taffy::{AvailableSpace, Size};

/// Trait for widgets that can measure their intrinsic content size.
///
/// This trait allows widgets to report their natural size based on content,
/// which helps the layout system make better decisions about sizing and
/// scrollbar visibility.
///
/// # Example
///
/// ```rust,no_run
/// use nptk_core::layout::measure::MeasureFunction;
/// use nptk_core::layout::Dimension;
/// use nalgebra::Vector2;
/// use taffy::{AvailableSpace, Size};
///
/// struct MyTextWidget {
///     text: String,
/// }
///
/// impl MeasureFunction for MyTextWidget {
///     fn measure(&self, constraints: Size<AvailableSpace>) -> Size<f32> {
///         // Measure text based on constraints
///         let max_width = match constraints.width {
///             AvailableSpace::Definite(w) => Some(w),
///             AvailableSpace::MinContent => None,
///             AvailableSpace::MaxContent => None,
///         };
///         
///         // Calculate text size...
///         Size {
///             width: 100.0,  // measured width
///             height: 20.0,  // measured height
///         }
///     }
/// }
/// ```
pub trait MeasureFunction {
    /// Measure the intrinsic size of this widget given available constraints.
    ///
    /// The constraints represent the maximum available space. The widget should
    /// return its preferred size within those constraints.
    ///
    /// # Parameters
    ///
    /// - `constraints`: The available space constraints from the parent
    ///
    /// # Returns
    ///
    /// The measured size of the widget's content. This should be the natural
    /// size the widget wants to be, respecting the constraints.
    fn measure(&self, constraints: Size<AvailableSpace>) -> Size<f32>;
}

/// Helper function to convert measured size to Vector2
pub fn measured_size_to_vector2(size: Size<f32>) -> Vector2<f32> {
    Vector2::new(size.width, size.height)
}

/// Helper function to create unbounded constraints for intrinsic measurement
pub fn unbounded_constraints() -> Size<AvailableSpace> {
    Size {
        width: AvailableSpace::MaxContent,
        height: AvailableSpace::MaxContent,
    }
}

/// Helper function to create definite constraints
pub fn definite_constraints(width: f32, height: f32) -> Size<AvailableSpace> {
    Size {
        width: AvailableSpace::Definite(width),
        height: AvailableSpace::Definite(height),
    }
}
