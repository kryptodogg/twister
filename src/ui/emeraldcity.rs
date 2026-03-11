use slint::Color;

pub const RESONANT_LOWER_HZ: f64 = 363.8;
pub const RESONANT_UPPER_HZ: f64 = 727.6;

const DEAD_R: u8 = 100;
const DEAD_G: u8 = 100;
const DEAD_B: u8 = 100;

// Flutopedia Resonance Palette stops (sRGB), aligned with the JSX prototype tokens.
const STOPS: [(f64, (u8, u8, u8)); 7] = [
    (0.0, (0xFF, 0x1A, 0x1A)), // Red
    (1.0 / 6.0, (0xFF, 0x66, 0x00)), // Orange
    (2.0 / 6.0, (0xFF, 0xAA, 0x00)), // Yellow
    (3.0 / 6.0, (0x22, 0xC5, 0x5E)), // Green
    (4.0 / 6.0, (0x00, 0xE5, 0xC8)), // Cyan/Teal
    (5.0 / 6.0, (0x00, 0x99, 0xFF)), // Blue
    (1.0, (0xA8, 0x55, 0xF7)), // Violet
];

/// Fold `freq_hz` into the base resonant octave `[RESONANT_LOWER_HZ, RESONANT_UPPER_HZ)`.
///
/// Returns `None` for invalid/dead signals (non-finite or <= 0.0).
pub fn resonant_fold_hz(freq_hz: f64) -> Option<f64> {
    if !freq_hz.is_finite() || freq_hz <= 0.0 {
        return None;
    }

    let mut f = freq_hz;

    // Fold up/down by octaves until f is within the base octave.
    while f < RESONANT_LOWER_HZ {
        f *= 2.0;
        if !f.is_finite() {
            return None;
        }
    }
    while f >= RESONANT_UPPER_HZ {
        f /= 2.0;
        if !f.is_finite() {
            return None;
        }
    }

    Some(f)
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let af = a as f64;
    let bf = b as f64;
    (af + (bf - af) * t.clamp(0.0, 1.0)).round().clamp(0.0, 255.0) as u8
}

/// Map `f_base` in `[lower, upper]` to an sRGB color across the visible spectrum.
///
/// The mapping is piecewise-linear across 7 stops: Red → Orange → Yellow → Green → Cyan → Blue → Violet.
pub fn frequency_to_rgb(f_base: f64, lower: f64, upper: f64) -> (u8, u8, u8) {
    if !f_base.is_finite() || !lower.is_finite() || !upper.is_finite() || upper <= lower {
        return (DEAD_R, DEAD_G, DEAD_B);
    }

    let mut t = (f_base - lower) / (upper - lower);
    t = t.clamp(0.0, 1.0);

    // Find the surrounding stop interval.
    for w in STOPS.windows(2) {
        let (t0, (r0, g0, b0)) = w[0];
        let (t1, (r1, g1, b1)) = w[1];
        if t <= t1 {
            let local = if (t1 - t0).abs() < f64::EPSILON {
                0.0
            } else {
                (t - t0) / (t1 - t0)
            };
            return (
                lerp_u8(r0, r1, local),
                lerp_u8(g0, g1, local),
                lerp_u8(b0, b1, local),
            );
        }
    }

    // Fallback: last stop.
    STOPS[STOPS.len() - 1].1
}

/// Compute the Emerald City octave-folded resonant color for a raw input frequency.
///
/// This is the ONLY dynamic sound-to-color mapping for Toto. Static palettes remain
/// for UI structure, borders, and text not driven by live frequency.
pub fn get_resonant_color(freq_hz: f64) -> Color {
    let Some(f_base) = resonant_fold_hz(freq_hz) else {
        return Color::from_rgb_u8(DEAD_R, DEAD_G, DEAD_B);
    };

    let (r, g, b) = frequency_to_rgb(f_base, RESONANT_LOWER_HZ, RESONANT_UPPER_HZ);
    Color::from_rgb_u8(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_signal_is_gray() {
        let c = get_resonant_color(0.0);
        assert!((c.red * 255.0 - DEAD_R as f32).abs() < 1.0);
        assert!((c.green * 255.0 - DEAD_G as f32).abs() < 1.0);
        assert!((c.blue * 255.0 - DEAD_B as f32).abs() < 1.0);
    }

    #[test]
    fn folds_into_base_octave() {
        let f = resonant_fold_hz(60.0).unwrap();
        assert!(f >= RESONANT_LOWER_HZ && f < RESONANT_UPPER_HZ);

        let f2 = resonant_fold_hz(2_400_000_000.0).unwrap();
        assert!(f2 >= RESONANT_LOWER_HZ && f2 < RESONANT_UPPER_HZ);
    }

    #[test]
    fn endpoints_map_to_expected_stops() {
        let (r0, g0, b0) = frequency_to_rgb(RESONANT_LOWER_HZ, RESONANT_LOWER_HZ, RESONANT_UPPER_HZ);
        assert_eq!((r0, g0, b0), STOPS[0].1);

        // Upper bound is exclusive in folding, but the mapper clamps.
        let (r1, g1, b1) = frequency_to_rgb(RESONANT_UPPER_HZ, RESONANT_LOWER_HZ, RESONANT_UPPER_HZ);
        assert_eq!((r1, g1, b1), STOPS[STOPS.len() - 1].1);
    }

    #[test]
    fn rgb_mapping_does_not_panic_for_large_values() {
        let _ = get_resonant_color(1.0e12);
    }
}

