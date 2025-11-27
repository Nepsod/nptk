#![cfg(target_os = "linux")]

//! Core Wayland protocol dispatch implementations.

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::globals::GlobalListContents;
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::shell::WaylandClientState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Currently unused - globals are bound via registry_queue_init
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _compositor: &wl_compositor::WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Compositor doesn't send events to clients
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _surface: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We do not currently react to wl_surface events
    }
}

impl Dispatch<wl_region::WlRegion, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _region: &wl_region::WlRegion,
        _event: wl_region::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_region has no events to handle
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _pool: &wl_shm_pool::WlShmPool,
        _event: wl_shm_pool::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events used
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _buffer: &wl_buffer::WlBuffer,
        _event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events used
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_shm::WlShm,
        _event: wl_shm::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // SHM doesn't send events to clients
    }
}

