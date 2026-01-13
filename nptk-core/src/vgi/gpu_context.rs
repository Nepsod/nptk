//! GPU context management for unified Instance and device handling.
//!
//! This module provides a GPUI-style GPU context that manages a single wgpu Instance
//! and provides device/adapter enumeration. This ensures all surfaces are created
//! with the same Instance that enumerates adapters, solving Instance mismatch issues
//! on Wayland.

use pollster;
use vello::wgpu;

/// Handle to a GPU device and its associated queue.
///
/// This is similar to `vello::util::DeviceHandle` but uses our own structure
/// to ensure compatibility with our GpuContext.
pub struct DeviceHandle {
    /// Adapter used to create the logical device.
    pub adapter: wgpu::Adapter,
    /// Logical device used for rendering.
    pub device: wgpu::Device,
    /// Queue associated with the logical device.
    pub queue: wgpu::Queue,
    /// Metadata describing the adapter.
    pub adapter_info: wgpu::AdapterInfo,
}

/// GPU context that manages a single wgpu Instance and device enumeration.
///
/// This follows GPUI's BladeContext pattern - a single Instance is created
/// and shared across all surfaces, ensuring compatibility on Wayland.
pub struct GpuContext {
    instance: wgpu::Instance,
    devices: Vec<DeviceHandle>,
}

impl GpuContext {
    /// Create a new GPU context.
    ///
    /// This creates a wgpu Instance and enumerates available adapters.
    /// Devices are not created yet - they must be created via `create_device_from_adapter()`.
    pub fn new() -> Result<Self, String> {
        log::debug!("Creating GPU context...");

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
        });

        log::debug!("GPU context created successfully");
        Ok(Self {
            instance,
            devices: Vec::new(),
        })
    }

    /// Get a reference to the wgpu Instance.
    ///
    /// This allows surfaces to be created with the same Instance that enumerates adapters.
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// Request an adapter with a compatible surface asynchronously.
    ///
    /// On Wayland, this is the recommended way to get an adapter that's compatible
    /// with the surface. The surface must be created with this context's Instance.
    ///
    /// # Arguments
    /// * `surface` - The wgpu surface to check compatibility with
    ///
    /// # Returns
    /// * `Some(Adapter)` if an adapter was found
    /// * `None` if no compatible adapter was found
    pub async fn request_adapter_with_surface_async(
        &self,
        surface: &wgpu::Surface<'static>,
    ) -> Option<wgpu::Adapter> {
        log::debug!("Requesting adapter with surface...");

        let adapter_result = self.instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        }).await;

        match adapter_result {
            Ok(adapter) => {
                log::debug!("Successfully requested adapter with surface");
                Some(adapter)
            },
            Err(err) => {
                log::warn!("No adapter found with surface: {:?}", err);
                None
            },
        }
    }

    /// Request an adapter with a compatible surface (blocking version for compatibility).
    ///
    /// On Wayland, this is the recommended way to get an adapter that's compatible
    /// with the surface. The surface must be created with this context's Instance.
    ///
    /// # Arguments
    /// * `surface` - The wgpu surface to check compatibility with
    ///
    /// # Returns
    /// * `Some(Adapter)` if an adapter was found
    /// * `None` if no compatible adapter was found
    pub fn request_adapter_with_surface(
        &self,
        surface: &wgpu::Surface<'static>,
    ) -> Option<wgpu::Adapter> {
        log::debug!("Requesting adapter with surface...");

        let adapter_result =
            pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            }));

        match adapter_result {
            Ok(adapter) => {
                log::debug!("Successfully requested adapter with surface");
                Some(adapter)
            },
            Err(err) => {
                log::warn!("No adapter found with surface: {:?}", err);
                None
            },
        }
    }

    /// Enumerate all available adapters.
    ///
    /// This can be used to find adapters without a surface, but on Wayland
    /// it's recommended to use `request_adapter_with_surface()` instead.
    pub fn enumerate_adapters(&self, backends: wgpu::Backends) -> Vec<wgpu::Adapter> {
        self.instance.enumerate_adapters(backends)
    }

    /// Create a device and queue from an adapter asynchronously.
    ///
    /// This creates a DeviceHandle that can be used for rendering.
    /// The device handle is NOT stored internally - use `add_device()` to store it
    /// if you want to retrieve it later via `enumerate_devices()`.
    ///
    /// # Arguments
    /// * `adapter` - The adapter to create a device from
    ///
    /// # Returns
    /// * `Ok(DeviceHandle)` if device creation succeeded
    /// * `Err(String)` if device creation failed
    pub async fn create_device_from_adapter(
        &mut self,
        adapter: &wgpu::Adapter,
    ) -> Result<DeviceHandle, String> {
        log::debug!("Creating device from adapter (async)...");

        let adapter_info = adapter.get_info();
        log::info!(
            "Selected GPU adapter: {} ({:?}, device_id=0x{:x}, vendor_id=0x{:x})",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device,
            adapter_info.vendor
        );

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("nptk-gpu-device"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::default(),
        }).await
        .map_err(|e| format!("Failed to create device: {:?}", e))?;

        let device_handle = DeviceHandle {
            adapter: adapter.clone(),
            device,
            queue,
            adapter_info,
        };

        log::debug!("Device created successfully (async)");
        Ok(device_handle)
    }

    /// Add a device handle to the internal list.
    ///
    /// This allows the device to be retrieved later via `enumerate_devices()`.
    /// Note: DeviceHandle cannot be cloned, so this consumes the handle.
    /// If you need to use the device after adding it, you'll need to get it
    /// from the devices list or keep a separate reference.
    pub fn add_device(&mut self, device_handle: DeviceHandle) {
        self.devices.push(device_handle);
    }

    /// Enumerate all available devices.
    ///
    /// Returns a slice of all DeviceHandles that have been added via `add_device()`.
    /// Initially, this will be empty until devices are added.
    pub fn enumerate_devices(&self) -> &[DeviceHandle] {
        &self.devices
    }

    /// Create a device from the first available adapter asynchronously.
    ///
    /// This is a convenience method that enumerates adapters and creates
    /// a device from the first one found. On Wayland, prefer using
    /// `request_adapter_with_surface()` followed by `create_device_from_adapter()`.
    ///
    /// Note: The device is NOT automatically added to the internal list.
    /// Call `add_device()` if you want to store it.
    pub async fn create_device_from_first_adapter(
        &mut self,
        backends: wgpu::Backends,
    ) -> Result<DeviceHandle, String> {
        let adapters = self.enumerate_adapters(backends);

        if adapters.is_empty() {
            return Err("No adapters found".to_string());
        }

        let adapter = adapters.first().unwrap();
        self.create_device_from_adapter(adapter).await
    }
}
