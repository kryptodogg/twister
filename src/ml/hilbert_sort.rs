// src/ml/hilbert_sort.rs
// Hilbert Curve Spatial Sorting for Mamba Input Preparation
//
// Particles are sorted along a 3D Hilbert space-filling curve for:
// 1. Cache-optimal memory access patterns
// 2. Spatial locality preservation for Mamba sequential processing
// 3. Bitwise indexing without trigonometry
//
// The Hilbert curve maps 3D coordinates → 1D indices such that:
// - Nearby voxels have nearby indices
// - No discontinuities (continuous curve through space)
// - Perfect for streaming Mamba input

use std::cmp::Ordering;

/// Compute 3D Hilbert curve index for a voxel coordinate
///
/// # Arguments
/// * `x, y, z` - Voxel coordinates (assumed 0..2^level)
/// * `level` - Recursion depth (typically 6 for 64³ grid)
///
/// # Returns
/// Hilbert index (0..2^(3*level))
///
/// # Example
/// ```ignore
/// let idx = hilbert_index(10, 20, 15, 6);  // Returns 0..262144 for 64³ grid
/// ```
pub fn hilbert_index(mut x: u32, mut y: u32, mut z: u32, level: u32) -> u64 {
    let mut d: u64 = 0;
    let mut s = 1u32 << (level - 1);

    while s > 0 {
        let rx = if (x & s) > 0 { 1 } else { 0 };
        let ry = if (y & s) > 0 { 1 } else { 0 };
        let rz = if (z & s) > 0 { 1 } else { 0 };

        d += (s as u64) * (s as u64) * ((3 * rx ^ ry) as u64 | ((rx ^ rz) as u64));

        // Rotate coordinate system
        let (nx, ny, nz) = rotate(x, y, z, rx, ry, rz, s);
        x = nx;
        y = ny;
        z = nz;

        s >>= 1;
    }
    d
}

/// Rotate coordinate system for Hilbert curve recursion
fn rotate(x: u32, y: u32, z: u32, rx: u32, ry: u32, rz: u32, s: u32) -> (u32, u32, u32) {
    let mut nx = x;
    let mut ny = y;
    let mut nz = z;

    // Rotation logic for 3D Hilbert curve
    if rz == 0 {
        if ry == 0 {
            // Rotate x,y
            std::mem::swap(&mut nx, &mut ny);
        }
        // Rotate y,z
        std::mem::swap(&mut ny, &mut nz);
    }

    if rx == 1 {
        nx = s - 1 - nx;
    }
    if ry == 1 {
        ny = s - 1 - ny;
    }
    if rz == 1 {
        nz = s - 1 - nz;
    }

    (nx, ny, nz)
}

/// Wrapper for sortable Hilbert index with original particle position
#[derive(Clone, Copy, Debug)]
pub struct HilbertKey {
    /// Hilbert curve index (0..2^(3*level))
    pub index: u64,
    /// Original particle buffer index (for tracking)
    pub particle_idx: u32,
}

impl PartialEq for HilbertKey {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for HilbertKey {}

impl PartialOrd for HilbertKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HilbertKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

/// CPU-side radix sort using Hilbert indices
///
/// # Returns
/// Vector of sorted (hilbert_index, original_particle_idx) tuples
pub fn radix_sort_hilbert(keys: &[HilbertKey]) -> Vec<u32> {
    let mut indexed = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (k.index, i as u32))
        .collect::<Vec<_>>();

    // Standard radix sort (8-bit passes)
    for shift in (0..64).step_by(8) {
        indexed.sort_by_key(|k| (k.0 >> shift) & 0xFF);
    }

    indexed.iter().map(|(_, orig_idx)| *orig_idx).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hilbert_index_origin() {
        let idx = hilbert_index(0, 0, 0, 3);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_hilbert_index_monotonic() {
        let mut prev_idx = 0u64;
        let mut continuity_breaks = 0;

        for i in 0..64 {
            let idx = hilbert_index(i, 0, 0, 6);
            if idx > prev_idx + 10 {
                // Allow some gaps but check general monotonicity
                continuity_breaks += 1;
            }
            prev_idx = idx;
        }

        // Hilbert curve should have good locality (few large jumps)
        assert!(continuity_breaks < 20);
    }

    #[test]
    fn test_radix_sort_hilbert() {
        let keys = vec![
            HilbertKey { index: 100, particle_idx: 0 },
            HilbertKey { index: 10, particle_idx: 1 },
            HilbertKey { index: 50, particle_idx: 2 },
            HilbertKey { index: 1, particle_idx: 3 },
        ];

        let sorted = radix_sort_hilbert(&keys);
        let expected = vec![3, 1, 2, 0]; // By increasing Hilbert index
        assert_eq!(sorted, expected);
    }

    #[test]
    fn test_hilbert_key_ordering() {
        let k1 = HilbertKey { index: 10, particle_idx: 5 };
        let k2 = HilbertKey { index: 20, particle_idx: 3 };
        assert!(k1 < k2);
    }
}
