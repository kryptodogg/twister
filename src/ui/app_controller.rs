// src/ui/app_controller.rs — Slint event dispatcher for device controls

use crate::app_state::DirtyFlags;
use crate::hardware_io::device_manager::{DeviceManager, DeviceStatus};
use slint::SharedString;
use std::sync::Arc;

/// Controller for device control callbacks.
///
/// Bridges Slint UI events to DeviceManager operations.
pub struct DeviceControlsController {
    device_manager: Arc<DeviceManager>,
    dirty_flags: Arc<DirtyFlags>,
}

impl DeviceControlsController {
    pub fn new(device_manager: Arc<DeviceManager>, dirty_flags: Arc<DirtyFlags>) -> Self {
        Self {
            device_manager,
            dirty_flags,
        }
    }

    /// Handle "+ Add RTL-SDR" button click.
    pub fn on_add_rtl_sdr_clicked(&self, device_index: u32) -> Result<SharedString, SharedString> {
        match self.device_manager.add_rtl_sdr(device_index) {
            Ok(device_id) => {
                self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);
                Ok(SharedString::from(format!(
                    "RTL-SDR device {} added (ID: {})",
                    device_index, device_id
                )))
            }
            Err(e) => Err(SharedString::from(format!("Failed to add RTL-SDR: {}", e))),
        }
    }

    /// Handle "+ Add Pluto+" button click (requires pluto-plus feature).
    #[cfg(feature = "pluto-plus")]
    pub fn on_add_pluto_clicked(&self, device_index: u32) -> Result<SharedString, SharedString> {
        match self.device_manager.add_pluto_plus(device_index) {
            Ok(device_id) => {
                self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);
                Ok(SharedString::from(format!(
                    "Pluto+ device added (ID: {})",
                    device_id
                )))
            }
            Err(e) => Err(SharedString::from(format!("Failed to add Pluto+: {}", e))),
        }
    }

    /// Handle "- Remove Device" button click.
    pub fn on_remove_device_clicked(&self, device_id: u32) -> Result<SharedString, SharedString> {
        match self.device_manager.remove_device(device_id) {
            Ok(_) => {
                self.dirty_flags.mark(&self.dirty_flags.device_list_dirty);
                Ok(SharedString::from(format!("Device {} removed", device_id)))
            }
            Err(e) => Err(SharedString::from(format!(
                "Failed to remove device: {}",
                e
            ))),
        }
    }

    /// Handle frequency input change.
    pub fn on_frequency_changed(
        &self,
        device_id: u32,
        freq_mhz: f32,
    ) -> Result<SharedString, SharedString> {
        let freq_hz = (freq_mhz * 1_000_000.0) as u64;
        match self.device_manager.tune_device(device_id, freq_hz) {
            Ok(_) => {
                self.dirty_flags
                    .mark(&self.dirty_flags.frequency_lock_dirty);
                Ok(SharedString::from(format!(
                    "Device {} tuned to {:.1} MHz",
                    device_id, freq_mhz
                )))
            }
            Err(e) => Err(SharedString::from(format!("Tuning failed: {}", e))),
        }
    }

    /// Get current device list for UI binding.
    pub fn get_device_list(&self) -> Vec<(u32, String, f32, String)> {
        self.device_manager
            .get_devices()
            .iter()
            .map(|handle| {
                let device_type = match handle.device_type {
                    crate::safe_sdr_wrapper::RadioDeviceType::RtlSdr => "RTL-SDR",
                    crate::safe_sdr_wrapper::RadioDeviceType::PlutoPlus => "Pluto+",
                };
                let status = match handle.status {
                    DeviceStatus::Ready => "Ready",
                    DeviceStatus::Tuning => "Tuning...",
                    DeviceStatus::Idle => "Idle",
                    DeviceStatus::Error => "Error",
                };
                (
                    handle.id,
                    format!("{} ({})", device_type, handle.id),
                    (handle.center_freq_hz as f32) / 1_000_000.0,
                    status.to_string(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        let dm = Arc::new(DeviceManager::new(Arc::new(DirtyFlags::new())));
        let flags = Arc::new(DirtyFlags::new());
        let _ctrl = DeviceControlsController::new(dm, flags);
        // Controller created successfully
    }
}
