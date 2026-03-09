// src/hardware_io/device_manager.rs — Device Lifecycle Orchestration
//
// Central registry for RadioDevice instances (RTL-SDR + Pluto+).
// Manages add/remove/tune operations via explicit UI callbacks (no auto-detect).
// Pushes state changes to AppState dirty flags for UI re-rendering.
//
// Thread-safe: All operations guarded by Arc<Mutex<>>.
// Zero allocation: Device list pre-allocated to MAX_DEVICES.

use crate::app_state::DirtyFlags;
use crate::safe_sdr_wrapper::{RadioDevice, RadioDeviceType};
use parking_lot::Mutex;
use std::sync::Arc;

pub const MAX_DEVICES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Idle,
    Tuning,
    Ready,
    Error,
}

#[derive(Debug, Clone)]
pub struct DeviceHandle {
    pub id: u32,
    pub device_type: RadioDeviceType,
    pub center_freq_hz: u64,
    pub sample_rate_hz: u32,
    pub status: DeviceStatus,
}

/// Central device registry.
///
/// All operations are explicit and tracked:
/// - add_device() → device added, dirty flag set
/// - remove_device() → device removed, dirty flag set
/// - tune_device() → frequency changed, dirty flag set
pub struct DeviceManager {
    // Active devices (Vec never reallocates; uses Option for empty slots)
    devices: Arc<Mutex<Vec<Option<RadioDevice>>>>,

    // Metadata for UI display (what we expose to Slint)
    handles: Arc<Mutex<Vec<DeviceHandle>>>,

    // Reference to shared dirty flags
    dirty_flags: Arc<DirtyFlags>,

    // Counter for next device ID (monotonically increasing)
    next_device_id: Arc<Mutex<u32>>,
}

impl DeviceManager {
    /// Create a new device manager with reference to AppState dirty flags.
    pub fn new(dirty_flags: Arc<DirtyFlags>) -> Self {
        DeviceManager {
            devices: Arc::new(Mutex::new(Vec::with_capacity(MAX_DEVICES))),
            handles: Arc::new(Mutex::new(Vec::with_capacity(MAX_DEVICES))),
            dirty_flags,
            next_device_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Explicitly add an RTL-SDR device.
    ///
    /// # Parameters
    /// - `device_index`: Physical USB enumeration index
    ///
    /// # Behavior
    /// 1. Opens RTL-SDR handle
    /// 2. Stores in devices registry
    /// 3. Creates DeviceHandle for UI
    /// 4. Sets device_list_dirty flag
    ///
    /// # Returns
    /// Assigned device ID (for future tune/remove ops)
    pub fn add_rtl_sdr(&self, device_index: u32) -> Result<u32, String> {
        let radio_dev = RadioDevice::open_rtl_sdr(device_index)?;

        let device_id = {
            let mut counter = self.next_device_id.lock();
            let id = *counter;
            *counter += 1;
            id
        };

        let handle = DeviceHandle {
            id: device_id,
            device_type: RadioDeviceType::RtlSdr,
            center_freq_hz: radio_dev.center_freq(),
            sample_rate_hz: radio_dev.sample_rate(),
            status: DeviceStatus::Ready,
        };

        {
            let mut devs = self.devices.lock();
            let mut hdls = self.handles.lock();

            if devs.len() >= MAX_DEVICES {
                return Err("Device registry full".into());
            }

            devs.push(Some(radio_dev));
            hdls.push(handle);
        }

        // Mark UI needs re-render
        self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);

        Ok(device_id)
    }

    /// Explicitly add a Pluto+ device.
    ///
    /// Only available when compiled with `--features pluto-plus`.
    #[cfg(feature = "pluto-plus")]
    pub fn add_pluto_plus(&self, device_id_param: u32) -> Result<u32, String> {
        let radio_dev = RadioDevice::open_pluto_plus(device_id_param)?;

        let device_id = {
            let mut counter = self.next_device_id.lock();
            let id = *counter;
            *counter += 1;
            id
        };

        let handle = DeviceHandle {
            id: device_id,
            device_type: RadioDeviceType::PlutoPlus,
            center_freq_hz: radio_dev.center_freq(),
            sample_rate_hz: radio_dev.sample_rate(),
            status: DeviceStatus::Ready,
        };

        {
            let mut devs = self.devices.lock();
            let mut hdls = self.handles.lock();

            if devs.len() >= MAX_DEVICES {
                return Err("Device registry full".into());
            }

            devs.push(Some(radio_dev));
            hdls.push(handle);
        }

        self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);

        Ok(device_id)
    }

    /// Remove a device by ID.
    ///
    /// # Behavior
    /// 1. Finds device in registry
    /// 2. Calls Drop (closes handle, frees resources)
    /// 3. Removes from both devices and handles vecs
    /// 4. Sets device_list_dirty flag
    pub fn remove_device(&self, device_id: u32) -> Result<(), String> {
        let mut devs = self.devices.lock();
        let mut hdls = self.handles.lock();

        // Find index by device_id
        let idx = hdls
            .iter()
            .position(|h| h.id == device_id)
            .ok_or("Device not found")?;

        // Drop device (triggers Drop impl, closes handle)
        devs[idx] = None;
        hdls.remove(idx);
        devs.remove(idx);

        self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);

        Ok(())
    }

    /// Tune a device to a new frequency.
    ///
    /// # Behavior
    /// 1. Finds device in registry
    /// 2. Sets status → Tuning
    /// 3. Calls tune_freq() on RadioDevice
    /// 4. Updates frequency in DeviceHandle
    /// 5. Sets status → Ready
    /// 6. Sets frequency_lock_dirty flag
    pub fn tune_device(&self, device_id: u32, freq_hz: u64) -> Result<(), String> {
        let mut devs = self.devices.lock();
        let mut hdls = self.handles.lock();

        // Find device
        let idx = hdls
            .iter()
            .position(|h| h.id == device_id)
            .ok_or("Device not found")?;

        // Update status
        hdls[idx].status = DeviceStatus::Tuning;

        // Perform tuning (mutable borrow)
        if let Some(Some(dev)) = devs.get_mut(idx) {
            dev.tune_freq(freq_hz)?;
        }

        // Update handle
        hdls[idx].center_freq_hz = freq_hz;
        hdls[idx].status = DeviceStatus::Ready;

        self.dirty_flags.mark(&self.dirty_flags.frequency_lock_dirty);

        Ok(())
    }

    /// Get snapshot of all active devices (for UI binding).
    ///
    /// Returns a clone of the handle list (cheap, Vec<DeviceHandle> is Copy).
    pub fn get_devices(&self) -> Vec<DeviceHandle> {
        self.handles.lock().clone()
    }

    /// Get mutable reference to a specific device for reading samples.
    ///
    /// Used by the Tokio dispatch loop to call read_sync().
    pub fn get_device_mut<F, R>(&self, device_id: u32, f: F) -> Result<R, String>
    where
        F: FnOnce(&RadioDevice) -> Result<R, String>,
    {
        let devs = self.devices.lock();

        let idx = self
            .handles
            .lock()
            .iter()
            .position(|h| h.id == device_id)
            .ok_or("Device not found")?;

        if let Some(Some(dev)) = devs.get(idx) {
            f(dev)
        } else {
            Err("Device handle is None".into())
        }
    }

    /// Check if any device is active.
    pub fn has_devices(&self) -> bool {
        !self.handles.lock().is_empty()
    }

    /// Get device count.
    pub fn device_count(&self) -> usize {
        self.handles.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_manager_new() {
        let flags = Arc::new(DirtyFlags::new());
        let mgr = DeviceManager::new(flags);
        assert_eq!(mgr.device_count(), 0);
        assert!(!mgr.has_devices());
    }

    #[test]
    fn test_device_removal_not_found() {
        let flags = Arc::new(DirtyFlags::new());
        let mgr = DeviceManager::new(flags);
        let result = mgr.remove_device(999);
        assert!(result.is_err());
    }
}
