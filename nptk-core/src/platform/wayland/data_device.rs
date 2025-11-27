#![cfg(target_os = "linux")]

//! Drag & drop support via wl_data_device_manager.

use std::sync::Arc;
use std::os::fd::OwnedFd;
use wayland_client::protocol::{
    wl_data_device, wl_data_device_manager, wl_data_offer, wl_data_source, wl_registry,
    wl_seat, wl_surface,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_client::backend::protocol::Message;

use super::shell::WaylandClientState;

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

#[derive(Debug)]
pub struct DataOffer {
    pub offer: wl_data_offer::WlDataOffer,
    pub mime_types: Vec<String>,
}

impl DataOffer {
    pub fn new(offer: wl_data_offer::WlDataOffer) -> Self {
        Self {
            offer,
            mime_types: Vec::new(),
        }
    }
}

pub struct DataDevice {
    pub device: wl_data_device::WlDataDevice,
    pub drag_offer: Option<DataOffer>,
    pub selection_offer: Option<DataOffer>,
}

impl DataDevice {
    pub fn new(device: wl_data_device::WlDataDevice) -> Self {
        Self {
            device,
            drag_offer: None,
            selection_offer: None,
        }
    }
}

impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        _manager: &wl_data_device_manager::WlDataDeviceManager,
        _event: wl_data_device_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // No events for manager
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        device: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_device::Event::DataOffer { id } => {
                let offer = DataOffer::new(id);
                // Store temporarily, will be moved to drag_offer or selection_offer
                // based on subsequent events (Enter or Selection)
                // For now we just track it in a pending list if needed, 
                // or we can rely on the fact that Enter/Selection comes right after.
                // But typically we need to store it to handle Offer events.
                state.pending_data_offers.push(offer);
            }
            wl_data_device::Event::Enter { serial: _, surface, x, y, id } => {
                // Drag enter
                if let Some(id) = id {
                    if let Some(index) = state.pending_data_offers.iter().position(|o| o.offer == id) {
                        let offer = state.pending_data_offers.remove(index);
                        if let Some(device_state) = state.data_devices.iter_mut().find(|d| d.device == *device) {
                            device_state.drag_offer = Some(offer);
                        }
                    }
                }
                // TODO: Handle drag enter logic (notify surface)
            }
            wl_data_device::Event::Leave => {
                // Drag leave
                if let Some(device_state) = state.data_devices.iter_mut().find(|d| d.device == *device) {
                    device_state.drag_offer = None;
                }
                // TODO: Handle drag leave logic
            }
            wl_data_device::Event::Motion { time: _, x, y } => {
                // Drag motion
                // TODO: Handle drag motion logic
            }
            wl_data_device::Event::Drop => {
                // Drag drop
                // TODO: Handle drop logic
            }
            wl_data_device::Event::Selection { id } => {
                // Clipboard selection
                if let Some(device_state) = state.data_devices.iter_mut().find(|d| d.device == *device) {
                    if let Some(id) = id {
                        if let Some(index) = state.pending_data_offers.iter().position(|o| o.offer == id) {
                            let offer = state.pending_data_offers.remove(index);
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

impl Dispatch<wl_data_offer::WlDataOffer, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        offer: &wl_data_offer::WlDataOffer,
        event: wl_data_offer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_offer::Event::Offer { mime_type } => {
                // Find the offer in pending or active offers and add mime type
                if let Some(o) = state.pending_data_offers.iter_mut().find(|o| o.offer == *offer) {
                    o.mime_types.push(mime_type);
                } else {
                    for device in &mut state.data_devices {
                        if let Some(o) = &mut device.drag_offer {
                            if o.offer == *offer {
                                o.mime_types.push(mime_type.clone());
                            }
                        }
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

impl Dispatch<wl_data_source::WlDataSource, ()> for WaylandClientState {
    fn event(
        _state: &mut Self,
        source: &wl_data_source::WlDataSource,
        event: wl_data_source::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_source::Event::Send { mime_type, fd } => {
                // TODO: Handle sending data
            }
            wl_data_source::Event::Cancelled => {
                source.destroy();
            }
            wl_data_source::Event::Target { mime_type } => {
                // Target accepted
            }
            wl_data_source::Event::DndDropPerformed => {
                // Drop performed
            }
            wl_data_source::Event::DndFinished => {
                source.destroy();
            }
            wl_data_source::Event::Action { dnd_action } => {
                // Action selected
            }
            _ => {}
        }
    }
}
