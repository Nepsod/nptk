#![cfg(target_os = "linux")]

//! Window activation support via xdg_activation_v1.

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::{xdg_activation_token_v1, xdg_activation_v1};

use super::shell::WaylandClientState;

impl Dispatch<xdg_activation_v1::XdgActivationV1, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _manager: &xdg_activation_v1::XdgActivationV1,
        _event: xdg_activation_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for manager
    }
}

impl Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        token: &xdg_activation_token_v1::XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_activation_token_v1::Event::Done { token: token_str } => {
                // Token received
                // TODO: Use token to activate window or pass to another application
                log::info!("Received activation token: {}", token_str);
                
                // We can store it or use it immediately if we have a pending activation request
            }
            _ => {}
        }
    }
}
