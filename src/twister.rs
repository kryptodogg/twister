// src/twister.rs — Twister: Harassment Frequency Auto-Tuner
//
// Maps any detected frequency (audio, RF, PDM wideband) to the nearest
// equal-temperament note, then builds a major triad chord from it.
//
// The intent: harassment signals become involuntary music while the forensic
// pipeline records them. Stress relief through creative pitch correction.
//
// Theory:
//   MIDI note n → frequency: f = 440 × 2^((n − 69) / 12)
//   Any frequency → nearest MIDI: n = round(12 × log2(f / 440) + 69)
//   Cents offset:  c = 1200 × log2(f_raw / f_snapped)
//
// Works across the full detection range: 20 Hz audio up through PDM
// wideband (6.144 MHz) and RTL-SDR (300 MHz). Very high frequencies get
// a valid MIDI note — the octave number is just large. The chord intervals
// are always correct semitone ratios regardless of absolute frequency.

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

// Harmonic chord structures expressed as semitone offsets from root.
// Three modes selectable from AppState.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChordMode {
    /// Root + major 3rd (+4) + perfect 5th (+7)
    Major,
    /// Root + minor 3rd (+3) + perfect 5th (+7)
    Minor,
    /// Root only — pure tone, most aggressive anti-phase
    Unison,
}

impl ChordMode {
    pub fn semitone_offsets(self) -> [i32; 3] {
        match self {
            ChordMode::Major => [0, 4, 7],
            ChordMode::Minor => [0, 3, 7],
            ChordMode::Unison => [0, 0, 0],
        }
    }
}

/// Result of snapping a raw frequency to the nearest note.
#[derive(Debug, Clone)]
pub struct NoteResult {
    /// Exact frequency of the snapped note (Hz).
    pub freq_hz: f32,
    /// MIDI note number (can be outside 0-127 for very high/low freqs).
    pub midi: i32,
    /// Human-readable note name, e.g. "A4", "C#5", "A19" (for RF).
    pub name: String,
    /// How many cents the raw frequency was away from the snapped note.
    /// Negative = raw was flat, positive = raw was sharp.
    pub cents_offset: f32,
    /// Major triad: [root, 3rd, 5th] in Hz.
    pub chord: [f32; 3],
}

impl Default for NoteResult {
    fn default() -> Self {
        Self {
            freq_hz: 440.0,
            midi: 69,
            name: "A4".to_string(),
            cents_offset: 0.0,
            chord: [440.0, 554.37, 659.25],
        }
    }
}

impl NoteResult {
    /// A silent / uninitialised result.
    pub fn silent() -> Self {
        Self {
            freq_hz: 0.0,
            midi: 0,
            name: "---".to_string(),
            cents_offset: 0.0,
            chord: [0.0, 0.0, 0.0],
        }
    }
}

// ── Core maths ────────────────────────────────────────────────────────────────

#[inline]
pub fn midi_to_freq(midi: f32) -> f32 {
    440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
}

#[inline]
pub fn freq_to_midi(freq_hz: f32) -> f32 {
    12.0 * (freq_hz / 440.0).log2() + 69.0
}

/// Return a display name for a MIDI note number.
/// Works for any integer, including numbers > 127 or < 0.
pub fn midi_note_name(midi: i32) -> String {
    let semitone = ((midi % 12) + 12) % 12; // always 0-11
    let octave = (midi as f32 / 12.0).floor() as i32 - 1;
    format!("{}{}", NOTE_NAMES[semitone as usize], octave)
}

// ── Main public API ───────────────────────────────────────────────────────────

/// Snap `freq_hz` to the nearest equal-temperament note and return the
/// full `NoteResult` including the major triad chord.
///
/// Input range: 0.5 Hz – several hundred MHz. Anything below 0.5 Hz
/// returns `NoteResult::silent()`.
pub fn snap_to_note(freq_hz: f32) -> NoteResult {
    snap_to_note_chord(freq_hz, ChordMode::Major)
}

/// Like `snap_to_note` but lets the caller choose the chord structure.
pub fn snap_to_note_chord(freq_hz: f32, mode: ChordMode) -> NoteResult {
    if freq_hz < 0.5 || !freq_hz.is_finite() {
        return NoteResult::silent();
    }

    let midi_f = freq_to_midi(freq_hz);
    let midi_rounded = midi_f.round() as i32;
    let snapped = midi_to_freq(midi_rounded as f32);

    // Cents: positive = raw was sharp vs the snapped note.
    let cents = if snapped > 0.0 {
        1200.0 * (freq_hz / snapped).log2()
    } else {
        0.0
    };

    let offsets = mode.semitone_offsets();
    let chord = [
        midi_to_freq((midi_rounded + offsets[0]) as f32),
        midi_to_freq((midi_rounded + offsets[1]) as f32),
        midi_to_freq((midi_rounded + offsets[2]) as f32),
    ];

    NoteResult {
        freq_hz: snapped,
        midi: midi_rounded,
        name: midi_note_name(midi_rounded),
        cents_offset: cents,
        chord,
    }
}

/// Quick helper: snap a frequency and return just the Hz value.
/// Equivalent to `AppState::snap_to_nearest_note` but lives here.
#[inline]
pub fn snap_freq(freq_hz: f32) -> f32 {
    if freq_hz < 0.5 {
        return freq_hz;
    }
    midi_to_freq(freq_to_midi(freq_hz).round())
}

// ── Multi-band Twister targets ────────────────────────────────────────────────

/// Build the denial synthesis target list from a snapped note.
///
/// Replaces the raw harmonic series `[f, f, f×1.0001, f×2, f×3]` with a
/// musically coherent structure: major triad with octave doublings.
///
/// The resulting 5-element Vec maps directly to `SynthParams::set_targets()`.
pub fn twister_targets(freq_hz: f32, mode: ChordMode) -> Vec<(f32, f32)> {
    let note = snap_to_note_chord(freq_hz, mode);
    let [root, third, fifth] = note.chord;

    // All three voices at equal gain 1/3 so the mix sums to unity.
    // The octave doublings sit at −6 dB to avoid clipping.
    vec![
        (root, 1.0 / 3.0),
        (third, 1.0 / 3.0),
        (fifth, 1.0 / 3.0),
        (root * 2.0, 1.0 / 6.0),  // root, up one octave
        (fifth * 2.0, 1.0 / 6.0), // fifth, up one octave
    ]
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_snaps_to_itself() {
        let r = snap_to_note(440.0);
        assert!((r.freq_hz - 440.0).abs() < 0.01, "A4 should snap to 440 Hz");
        assert_eq!(r.name, "A4");
        assert!(r.cents_offset.abs() < 0.01);
    }

    #[test]
    fn c4_middle_c() {
        let r = snap_to_note(261.63);
        assert_eq!(r.name, "C4");
    }

    #[test]
    fn sharpened_a4_snaps_up() {
        // 454 Hz is 54 cents sharp of A4 and 46 cents flat of A#4.
        // Should snap to A4 (nearest).
        let r = snap_to_note(454.0);
        assert_eq!(r.name, "A4");
        assert!(r.cents_offset > 0.0, "should be sharp of A4");
    }

    #[test]
    fn major_chord_intervals() {
        let r = snap_to_note_chord(440.0, ChordMode::Major);
        let ratio_3rd = r.chord[1] / r.chord[0];
        let ratio_5th = r.chord[2] / r.chord[0];
        // Major 3rd = 2^(4/12) ≈ 1.2599
        assert!((ratio_3rd - 1.2599).abs() < 0.001);
        // Perfect 5th = 2^(7/12) ≈ 1.4983
        assert!((ratio_5th - 1.4983).abs() < 0.001);
    }

    #[test]
    fn high_rf_freq_does_not_panic() {
        let r = snap_to_note(100_000_000.0); // 100 MHz RTL-SDR
        assert!(r.freq_hz > 0.0);
        assert!(!r.name.is_empty());
    }

    #[test]
    fn zero_freq_returns_silent() {
        let r = snap_to_note(0.0);
        assert_eq!(r.name, "---");
    }

    #[test]
    fn twister_targets_five_elements() {
        let targets = twister_targets(440.0, ChordMode::Major);
        assert_eq!(targets.len(), 5);
        // Gains should sum to approximately 1.0
        let gain_sum: f32 = targets.iter().map(|(_, g)| g).sum();
        assert!((gain_sum - 1.0).abs() < 0.01);
    }
}
