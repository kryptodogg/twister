use crate::dispatch::backend::{SignalBackend, BackendError};
use crate::ml::field_particle::FieldParticle;

pub struct HetSynthesizer {
    backends: Vec<Box<dyn SignalBackend>>,
    oscillators: Vec<Oscillator>,
}

struct Oscillator {
    freq_hz: f32,
    phase: f32,
    amplitude: f32,
}

impl HetSynthesizer {
    pub fn new() -> Self {
        let mut oscillators = Vec::with_capacity(12);
        for _ in 0..12 {
            oscillators.push(Oscillator {
                freq_hz: 0.0,
                phase: 0.0,
                amplitude: 0.0,
            });
        }
        Self {
            backends: Vec::new(),
            oscillators,
        }
    }

    pub fn add_backend(&mut self, backend: Box<dyn SignalBackend>) {
        self.backends.push(backend);
    }

    pub fn process_particle(&mut self, particle: &FieldParticle) {
        let bucket = particle.material_id as usize % 12;

        // Octave folding to 20-1000 Hz range
        let mut folded_freq = particle.freq_hz as f32;
        if folded_freq > 0.0 {
            while folded_freq > 1000.0 {
                folded_freq /= 2.0;
            }
            while folded_freq < 20.0 && folded_freq > 0.0 {
                folded_freq *= 2.0;
            }
        }

        self.oscillators[bucket].freq_hz = folded_freq;
        self.oscillators[bucket].amplitude = particle.energy;
    }

    pub fn generate_samples(&mut self, num_samples: usize, sample_rate: f32) -> Vec<f32> {
        let mut samples = vec![0.0; num_samples];
        let dt = 1.0 / sample_rate;

        for i in 0..num_samples {
            let mut mix = 0.0;
            for osc in &mut self.oscillators {
                if osc.amplitude > 0.0 {
                    mix += osc.amplitude * (osc.phase * 2.0 * std::f32::consts::PI).sin();
                    osc.phase = (osc.phase + osc.freq_hz * dt) % 1.0;
                }
            }
            samples[i] = mix / 12.0; // Normalize
        }

        // Write to backends
        for backend in &mut self.backends {
            let _ = backend.write_pcm(&samples);
        }

        samples
    }
}
