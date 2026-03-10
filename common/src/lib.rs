//! Common types and utilities for Project Synesthesia.

/// A payload structure for transmitting haptic data and metadata.
/// This structure is meticulously aligned to 128 bytes to match
/// modern CPU cache lines for deterministic, low-latency performance.
#[repr(C, align(128))]
pub struct HeterodynePayload {
    /// 64 bytes of PDM-encoded haptic data.
    pub haptic_data: [u8; 64],
    /// A 64-bit metadata field for source ID, frequency, etc.
    pub metadata: u64,
    /// A 64-bit high-precision timestamp.
    pub timestamp: u64,
}

// Assert that the size and alignment are correct at compile time.
const _: () = {
    assert!(size_of::<HeterodynePayload>() == 128);
    assert!(align_of::<HeterodynePayload>() == 128);
};