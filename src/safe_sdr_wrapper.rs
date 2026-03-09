// src/safe_sdr_wrapper.rs — Safe wrappers for RTL-SDR and Pluto+ FFI

use crate::rtlsdr_ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioDeviceType {
    RtlSdr,
    PlutoPlus,
}

/// Safe handle to an open RTL-SDR or Pluto+ device.
/// All FFI operations are unsafe and contained within this struct.
pub struct RadioDevice {
    device_type: RadioDeviceType,
    device_id: u32,
    center_freq_hz: u64,
    sample_rate_hz: u32,

    // RTL-SDR handle (Option because Pluto+ doesn't use it)
    rtl_handle: Option<*mut rtlsdr_ffi::rtlsdr_dev_t>,

    // Pluto+ handles (Feature-gated, optional)
    #[cfg(feature = "pluto-plus")]
    pluto_context: Option<*mut rtlsdr_ffi::iio_context>,
    #[cfg(feature = "pluto-plus")]
    pluto_device: Option<*mut rtlsdr_ffi::iio_device>,
}

impl RadioDevice {
    /// Open an RTL-SDR device by USB enumeration index.
    ///
    /// # Parameters
    /// - `device_index`: USB enumeration index (0, 1, 2, ...)
    ///
    /// # Returns
    /// - `Ok(RadioDevice)` on success, initialized with safe defaults
    /// - `Err(String)` on FFI failure
    ///
    /// # Defaults set
    /// - Sample rate: 2.4 MSPS
    /// - Center frequency: 2.4 GHz
    /// - Tuner gain mode: Manual
    /// - AGC mode: On
    pub fn open_rtl_sdr(device_index: u32) -> Result<Self, String> {
        unsafe {
            let mut handle: *mut rtlsdr_ffi::rtlsdr_dev_t = std::ptr::null_mut();
            let ret = rtlsdr_ffi::rtlsdr_open(&mut handle, device_index);

            if ret != rtlsdr_ffi::RTLSDR_SUCCESS || handle.is_null() {
                return Err(format!(
                    "Failed to open RTL-SDR device {}: code {}",
                    device_index, ret
                ));
            }

            // Safe defaults
            rtlsdr_ffi::rtlsdr_set_sample_rate(handle, 2_400_000);
            rtlsdr_ffi::rtlsdr_set_center_freq(handle, 2_400_000_000);
            rtlsdr_ffi::rtlsdr_set_tuner_gain_mode(handle, 1);
            rtlsdr_ffi::rtlsdr_set_agc_mode(handle, 1);

            Ok(RadioDevice {
                device_type: RadioDeviceType::RtlSdr,
                device_id: device_index,
                center_freq_hz: 2_400_000_000,
                sample_rate_hz: 2_400_000,
                rtl_handle: Some(handle),
                #[cfg(feature = "pluto-plus")]
                pluto_context: None,
                #[cfg(feature = "pluto-plus")]
                pluto_device: None,
            })
        }
    }

    /// Open a Pluto+ device (requires `--features pluto-plus`).
    ///
    /// Discovers ad9361-phy device on the USB context.
    #[cfg(feature = "pluto-plus")]
    pub fn open_pluto_plus(device_id: u32) -> Result<Self, String> {
        unsafe {
            let ctx = rtlsdr_ffi::iio_create_default_context();
            if ctx.is_null() {
                return Err("Failed to create libiio context".into());
            }

            let device_name = CString::new("ad9361-phy").map_err(|_| "Invalid device name")?;
            let dev = rtlsdr_ffi::iio_context_find_device(ctx, device_name.as_ptr());
            if dev.is_null() {
                rtlsdr_ffi::iio_context_destroy(ctx);
                return Err("ad9361 device not found on Pluto+".into());
            }

            Ok(RadioDevice {
                device_type: RadioDeviceType::PlutoPlus,
                device_id,
                center_freq_hz: 2_400_000_000,
                sample_rate_hz: 2_000_000,
                rtl_handle: None,
                pluto_context: Some(ctx),
                pluto_device: Some(dev),
            })
        }
    }

    /// Tune device to a specific frequency (Hz).
    ///
    /// # Parameters
    /// - `freq_hz`: Frequency in Hz (e.g., 2_400_000_000 for 2.4 GHz)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(String)` if frequency set failed
    pub fn tune_freq(&mut self, freq_hz: u64) -> Result<(), String> {
        match self.device_type {
            RadioDeviceType::RtlSdr => unsafe {
                let ret = rtlsdr_ffi::rtlsdr_set_center_freq(
                    self.rtl_handle.ok_or("No RTL-SDR handle")?,
                    freq_hz as u32,
                );
                if ret != rtlsdr_ffi::RTLSDR_SUCCESS {
                    return Err(format!("RTL-SDR freq set failed: {}", ret));
                }
            },
            RadioDeviceType::PlutoPlus => {
                #[cfg(feature = "pluto-plus")]
                unsafe {
                    let attr_name = CString::new("RX_LO").map_err(|_| "Invalid attr name")?;
                    let ret = rtlsdr_ffi::iio_device_attr_write_longlong(
                        self.pluto_device.ok_or("No Pluto+ device")?,
                        attr_name.as_ptr(),
                        freq_hz as i64,
                    );
                    if ret < 0 {
                        return Err(format!("Pluto+ freq set failed: {}", ret));
                    }
                }
                #[cfg(not(feature = "pluto-plus"))]
                {
                    return Err("Pluto+ support not compiled in".into());
                }
            }
        }

        self.center_freq_hz = freq_hz;
        Ok(())
    }

    /// Read IQ samples from device (blocking, RTL-SDR only).
    ///
    /// # Parameters
    /// - `buffer`: Mutable u8 buffer (will be filled with raw IQ samples)
    ///
    /// # Returns
    /// - `Ok(n_read)` — number of bytes read
    /// - `Err(String)` on I/O error
    ///
    /// # Note
    /// Only RTL-SDR supports sync reads. Pluto+ uses async I/O (future feature).
    pub fn read_sync(&self, buffer: &mut [u8]) -> Result<usize, String> {
        if self.device_type != RadioDeviceType::RtlSdr {
            return Err("Only RTL-SDR supports sync reads (for now)".into());
        }

        unsafe {
            let mut n_read: i32 = 0;
            let ret = rtlsdr_ffi::rtlsdr_read_sync(
                self.rtl_handle.ok_or("No RTL-SDR handle")?,
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                &mut n_read,
            );
            if ret != rtlsdr_ffi::RTLSDR_SUCCESS {
                return Err(format!("RTL-SDR read failed: {}", ret));
            }
            Ok(n_read as usize)
        }
    }

    // Accessors
    pub fn device_type(&self) -> RadioDeviceType {
        self.device_type
    }

    pub fn center_freq(&self) -> u64 {
        self.center_freq_hz
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate_hz
    }
}

impl Drop for RadioDevice {
    fn drop(&mut self) {
        unsafe {
            if let Some(handle) = self.rtl_handle.take() {
                let _ = rtlsdr_ffi::rtlsdr_close(handle);
            }
            #[cfg(feature = "pluto-plus")]
            {
                if let Some(ctx) = self.pluto_context.take() {
                    rtlsdr_ffi::iio_context_destroy(ctx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radio_device_type_enum() {
        let rt = RadioDeviceType::RtlSdr;
        assert_eq!(rt, RadioDeviceType::RtlSdr);
        assert_ne!(rt, RadioDeviceType::PlutoPlus);
    }

    #[test]
    fn test_radio_device_freq_update() {
        // This test would require actual hardware or mocking.
        // For now, just ensure the enum/type system works.
        let _device_type = RadioDeviceType::RtlSdr;
    }
}
