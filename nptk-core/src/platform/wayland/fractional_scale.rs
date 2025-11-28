#![cfg(target_os = "linux")]

//! Fractional scaling support via wp_fractional_scale_manager_v1.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
};

use super::shell::WaylandClientState;

/// Wrapper around the Wayland fractional scale object.
pub struct FractionalScale {
    /// The underlying Wayland fractional scale object.
    pub object: wp_fractional_scale_v1::WpFractionalScaleV1,
}

impl FractionalScale {
    /// Create a new FractionalScale wrapper.
    pub fn new(object: wp_fractional_scale_v1::WpFractionalScaleV1) -> Self {
        Self { object }
    }
}

impl Dispatch<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _manager: &wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        _event: wp_fractional_scale_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for manager
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _scale: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wp_fractional_scale_v1::Event::PreferredScale { scale: preferred_scale } => {
                // preferred_scale is in 120ths of 1
                let _scale_factor = preferred_scale as f64 / 120.0;
                
                // Find the window associated with this scale object and update its scale factor
                // This requires tracking which window owns this scale object.
                // For now, we'll just log it or store it if we can map it back.
                // In a real implementation, we'd store the scale object in the Window struct
                // and update the window's scale factor here.
                
                // TODO: Update window scale factor
            }
            _ => {}
        }
    }
}
