// src/rtlsdr_ffi.rs — FFI Bindings to librtlsdr
//
// Gated behind the `rtlsdr` Cargo feature so the project compiles and links
// on any machine regardless of whether librtlsdr is installed.
//
// To build with hardware support:
//   cargo build --features rtlsdr
//
// NixOS: add `pkgs.rtl-sdr` to your devShell packages.
// The #[link(name="rtlsdr")] attribute names the shared library; pkg-config
// resolves the path automatically when the package is in scope.
//
// Safety: All functions are unsafe due to FFI.
// The safe wrapper in rtlsdr.rs provides memory-safe abstractions over these.
//
// NOTE (Windows): Linking against rtlsdr.lib is handled in build.rs for the
// "rtlsdr" feature, ensuring the project compiles on Windows even if the
// library is not in the system PATH.

use std::os::raw::{c_int, c_uint, c_void};

/// Opaque handle to RTL-SDR device.
#[repr(C)]
pub struct rtlsdr_dev_t {
    _private: [u8; 0],
}

pub type RtlSdrResult = c_int;

pub const RTLSDR_SUCCESS: c_int = 0;
pub const RTLSDR_ERROR_DEVICE_NOT_FOUND: c_int = -1;
pub const RTLSDR_ERROR_HW_ERROR: c_int = -2;
pub const RTLSDR_ERROR_INVALID_ARG: c_int = -4;

pub type RtlSdrSample = u8;

#[allow(non_camel_case_types)]
pub type rtlsdr_read_async_cb_t =
    extern "C" fn(buf: *mut RtlSdrSample, len: c_uint, ctx: *mut c_void);

// ── Live FFI (compiled only with --features rtlsdr) ──────────────────────────
#[cfg(feature = "rtlsdr")]
#[link(name = "rtlsdr")]
unsafe extern "C" {
    pub fn rtlsdr_get_device_count() -> u32;
    pub fn rtlsdr_get_device_name(index: u32) -> *const c_int;
    pub fn rtlsdr_open(dev: *mut *mut rtlsdr_dev_t, index: u32) -> RtlSdrResult;
    pub fn rtlsdr_close(dev: *mut rtlsdr_dev_t) -> RtlSdrResult;
    pub fn rtlsdr_reset_buffer(dev: *mut rtlsdr_dev_t) -> RtlSdrResult;
    pub fn rtlsdr_set_center_freq(dev: *mut rtlsdr_dev_t, freq_hz: u32) -> RtlSdrResult;
    pub fn rtlsdr_get_center_freq(dev: *mut rtlsdr_dev_t) -> u32;
    pub fn rtlsdr_set_sample_rate(dev: *mut rtlsdr_dev_t, rate_hz: u32) -> RtlSdrResult;
    pub fn rtlsdr_get_sample_rate(dev: *mut rtlsdr_dev_t) -> u32;
    pub fn rtlsdr_set_tuner_gain_mode(dev: *mut rtlsdr_dev_t, manual: c_int) -> RtlSdrResult;
    pub fn rtlsdr_set_tuner_gain(dev: *mut rtlsdr_dev_t, gain: c_int) -> RtlSdrResult;
    pub fn rtlsdr_set_agc_mode(dev: *mut rtlsdr_dev_t, on: c_int) -> RtlSdrResult;
    pub fn rtlsdr_set_direct_sampling(dev: *mut rtlsdr_dev_t, on: c_int) -> RtlSdrResult;
    pub fn rtlsdr_read_sync(
        dev: *mut rtlsdr_dev_t,
        buf: *mut RtlSdrSample,
        len: c_int,
        n_read: *mut c_int,
    ) -> RtlSdrResult;
    pub fn rtlsdr_read_async(
        dev: *mut rtlsdr_dev_t,
        cb: rtlsdr_read_async_cb_t,
        ctx: *mut c_void,
        buf_num: c_uint,
        buf_len: c_uint,
    ) -> RtlSdrResult;
    pub fn rtlsdr_cancel_async(dev: *mut rtlsdr_dev_t) -> RtlSdrResult;
}

// ── Stubs (compiled when hardware feature is OFF) ─────────────────────────────
// Allows rtlsdr.rs and sdr.rs to compile unconditionally.
// At runtime the SDR engine detects zero devices and disables itself cleanly.

#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_get_device_count() -> u32 {
    0
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_get_device_name(_i: u32) -> *const c_int {
    std::ptr::null()
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_open(_d: *mut *mut rtlsdr_dev_t, _i: u32) -> RtlSdrResult {
    RTLSDR_ERROR_DEVICE_NOT_FOUND
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_close(_d: *mut rtlsdr_dev_t) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_reset_buffer(_d: *mut rtlsdr_dev_t) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_set_center_freq(_d: *mut rtlsdr_dev_t, _f: u32) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_get_center_freq(_d: *mut rtlsdr_dev_t) -> u32 {
    0
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_set_sample_rate(_d: *mut rtlsdr_dev_t, _r: u32) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_get_sample_rate(_d: *mut rtlsdr_dev_t) -> u32 {
    0
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_set_tuner_gain_mode(_d: *mut rtlsdr_dev_t, _m: c_int) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_set_tuner_gain(_d: *mut rtlsdr_dev_t, _g: c_int) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_set_agc_mode(_d: *mut rtlsdr_dev_t, _on: c_int) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_set_direct_sampling(_d: *mut rtlsdr_dev_t, _on: c_int) -> RtlSdrResult {
    RTLSDR_SUCCESS
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_read_sync(
    _d: *mut rtlsdr_dev_t,
    _b: *mut RtlSdrSample,
    _l: c_int,
    _n: *mut c_int,
) -> RtlSdrResult {
    RTLSDR_ERROR_DEVICE_NOT_FOUND
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_read_async(
    _d: *mut rtlsdr_dev_t,
    _cb: rtlsdr_read_async_cb_t,
    _ctx: *mut c_void,
    _bn: c_uint,
    _bl: c_uint,
) -> RtlSdrResult {
    RTLSDR_ERROR_DEVICE_NOT_FOUND
}
#[cfg(not(feature = "rtlsdr"))]
pub unsafe fn rtlsdr_cancel_async(_d: *mut rtlsdr_dev_t) -> RtlSdrResult {
    RTLSDR_SUCCESS
}

// ── Helpers (always compiled) ─────────────────────────────────────────────────

pub fn rtl_error_to_string(err: RtlSdrResult) -> &'static str {
    match err {
        RTLSDR_SUCCESS => "Success",
        RTLSDR_ERROR_DEVICE_NOT_FOUND => "Device not found",
        RTLSDR_ERROR_HW_ERROR => "Hardware error",
        RTLSDR_ERROR_INVALID_ARG => "Invalid argument",
        _ => "Unknown error",
    }
}

#[inline]
pub fn is_rtl_success(err: RtlSdrResult) -> bool {
    err == RTLSDR_SUCCESS
}

#[cfg(feature = "pluto-plus")]
#[allow(non_camel_case_types)]
pub type iio_context = c_void;
#[cfg(feature = "pluto-plus")]
#[allow(non_camel_case_types)]
pub type iio_device = c_void;
#[cfg(not(feature = "pluto-plus"))]
#[allow(non_camel_case_types)]
pub type iio_context = c_void;
#[cfg(not(feature = "pluto-plus"))]
#[allow(non_camel_case_types)]
pub type iio_device = c_void;

use std::os::raw::c_char;

#[cfg(feature = "pluto-plus")]
#[link(name = "iio")]
unsafe extern "C" {
    pub fn iio_create_default_context() -> *mut iio_context;
    pub fn iio_context_destroy(ctx: *mut iio_context);
    pub fn iio_context_find_device(ctx: *const iio_context, name: *const c_char)
    -> *mut iio_device;
    pub fn iio_device_attr_write_longlong(
        dev: *mut iio_device,
        attr: *const c_char,
        val: i64,
    ) -> c_int;
}

#[cfg(not(feature = "pluto-plus"))]
pub unsafe fn iio_create_default_context() -> *mut iio_context {
    std::ptr::null_mut()
}
#[cfg(not(feature = "pluto-plus"))]
pub unsafe fn iio_context_destroy(_ctx: *mut iio_context) {}
#[cfg(not(feature = "pluto-plus"))]
pub unsafe fn iio_context_find_device(
    _ctx: *const iio_context,
    _name: *const c_char,
) -> *mut iio_device {
    std::ptr::null_mut()
}
#[cfg(not(feature = "pluto-plus"))]
pub unsafe fn iio_device_attr_write_longlong(
    _dev: *mut iio_device,
    _attr: *const c_char,
    _val: i64,
) -> c_int {
    -1
}
