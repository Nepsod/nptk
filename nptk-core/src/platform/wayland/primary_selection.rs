#![cfg(target_os = "linux")]

//! Primary selection support via zwp_primary_selection_device_manager_v1.

use std::sync::Arc;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::primary_selection::zv1::client::{
    zwp_primary_selection_device_manager_v1, zwp_primary_selection_device_v1,
    zwp_primary_selection_offer_v1, zwp_primary_selection_source_v1,
};

use super::shell::WaylandClientState;
use wayland_client::backend::protocol::Message;

// Dummy ObjectData implementation for event_created_child
struct DummyObjectData;

impl wayland_client::backend::ObjectData for DummyObjectData {
    fn event(
        self: Arc<Self>,
        _backend: &wayland_client::backend::Backend,
        _msg: Message<wayland_client::backend::ObjectId, std::os::fd::OwnedFd>,
    ) -> Option<Arc<dyn wayland_client::backend::ObjectData>> {
        None
    }

    fn destroyed(
        &self,
        _object_id: wayland_client::backend::ObjectId,
    ) {
    }
}

/// Wrapper around a Wayland primary selection offer.
#[derive(Debug)]
pub struct PrimaryDataOffer {
    /// The underlying Wayland primary selection offer object.
    pub offer: zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1,
    /// List of MIME types offered.
    pub mime_types: Vec<String>,
}

impl PrimaryDataOffer {
    /// Create a new PrimaryDataOffer wrapper.
    pub fn new(offer: zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1) -> Self {
        Self {
            offer,
            mime_types: Vec::new(),
        }
    }
}

/// Wrapper around a Wayland primary selection device.
pub struct PrimarySelectionDevice {
    /// The underlying Wayland primary selection device object.
    pub device: zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
    /// The current primary selection offer, if any.
    pub selection_offer: Option<PrimaryDataOffer>,
}

impl PrimarySelectionDevice {
    /// Create a new PrimarySelectionDevice wrapper.
    pub fn new(device: zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1) -> Self {
        Self {
            device,
            selection_offer: None,
        }
    }
}

impl Dispatch<zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        _manager: &zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
        _event: zwp_primary_selection_device_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for manager
    }
}

impl Dispatch<zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1, ()>
    for WaylandClientState
{
    fn event(
        state: &mut Self,
        device: &zwp_primary_selection_device_v1::ZwpPrimarySelectionDeviceV1,
        event: zwp_primary_selection_device_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_primary_selection_device_v1::Event::DataOffer { offer } => {
                let offer = PrimaryDataOffer::new(offer);
                state.pending_primary_offers.push(offer);
            }
            zwp_primary_selection_device_v1::Event::Selection { id } => {
                if let Some(device_state) = state.primary_selection_devices.iter_mut().find(|d| d.device == *device) {
                    if let Some(id) = id {
                        if let Some(index) = state.pending_primary_offers.iter().position(|o| o.offer == id) {
                            let offer = state.pending_primary_offers.remove(index);
                            device_state.selection_offer = Some(offer);
                        }
                    } else {
                        device_state.selection_offer = None;
                    }
                }
            }
            _ => {}
        }
    }

    fn event_created_child(
        _opcode: u16,
        _qhandle: &QueueHandle<Self>,
    ) -> Arc<dyn wayland_client::backend::ObjectData + 'static> {
        Arc::new(DummyObjectData)
    }
}

impl Dispatch<zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1, ()>
    for WaylandClientState
{
    fn event(
        state: &mut Self,
        offer: &zwp_primary_selection_offer_v1::ZwpPrimarySelectionOfferV1,
        event: zwp_primary_selection_offer_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_primary_selection_offer_v1::Event::Offer { mime_type } => {
                if let Some(o) = state.pending_primary_offers.iter_mut().find(|o| o.offer == *offer) {
                    o.mime_types.push(mime_type);
                } else {
                    for device in &mut state.primary_selection_devices {
                        if let Some(o) = &mut device.selection_offer {
                            if o.offer == *offer {
                                o.mime_types.push(mime_type.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1, ()>
    for WaylandClientState
{
    fn event(
        _state: &mut Self,
        source: &zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1,
        event: zwp_primary_selection_source_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_primary_selection_source_v1::Event::Send { mime_type: _, fd: _ } => {
                // TODO: Handle sending data
            }
            zwp_primary_selection_source_v1::Event::Cancelled => {
                source.destroy();
            }
            _ => {}
        }
    }
}
