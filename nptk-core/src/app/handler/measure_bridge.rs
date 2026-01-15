// SPDX-License-Identifier: LGPL-3.0-only
use taffy::{AvailableSpace, Size};

/// Create a Taffy measure function from a widget's measure result.
///
/// This bridges between Widget::measure() (which returns Option<Size<f32>>)
/// and Taffy's measure function API (which requires Size<f32>).
///
/// If the widget doesn't provide a measure function (returns None),
/// this will return a default size based on constraints.
pub fn create_taffy_measure_func<F>(measure_fn: F) -> Box<dyn Fn(Size<AvailableSpace>) -> Size<f32> + Send + Sync>
where
    F: Fn(Size<AvailableSpace>) -> Option<Size<f32>> + Send + Sync + 'static,
{
    Box::new(move |constraints: Size<AvailableSpace>| -> Size<f32> {
        if let Some(size) = measure_fn(constraints) {
            size
        } else {
            // Fallback: use constraints to determine size
            let width = match constraints.width {
                AvailableSpace::Definite(w) if w > 0.0 => w,
                AvailableSpace::Definite(_) => 0.0,
                AvailableSpace::MinContent => 0.0,
                AvailableSpace::MaxContent => f32::INFINITY,
            };
            let height = match constraints.height {
                AvailableSpace::Definite(h) if h > 0.0 => h,
                AvailableSpace::Definite(_) => 0.0,
                AvailableSpace::MinContent => 0.0,
                AvailableSpace::MaxContent => f32::INFINITY,
            };
            Size { width, height }
        }
    })
}
