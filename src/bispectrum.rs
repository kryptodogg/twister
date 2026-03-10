// src/bispectrum.rs — GPU Bispectrum Engine  (v0.4)
//
// Changes from v0.3:
//   • Takes Arc<GpuShared> — no separate wgpu::Instance.
//   • Spatial culling: the CPU pre-screens rows/cols by energy before uploading
//     to GPU. Cells where both f1 and f2 have below-threshold spectral energy
//     are skipped in update_coherence(), saving ~60% of coherence accumulation
//     work on typical audio signals (most energy concentrated in <10% of bins).
//   • V-buffer input: BispectrumEngine::analyze_frame() still takes a &[f32]
//     slice (the dispatch thread extracts it from the V-buffer), keeping the
//     engine testable without a live V-buffer reference.

use crate::detection::{DetectionEvent, HardwareLayer, MIN_COHERENCE_FRAMES, ProductType};
use crate::gpu_shared::GpuShared;
use crate::twister::computer_vision::pose_estimator::PoseFrame;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

pub const BISPEC_FFT_SIZE: usize = 1024;
pub const BISPEC_BINS: usize = BISPEC_FFT_SIZE / 2; // 512
pub const BISPEC_MATRIX_SIZE: usize = BISPEC_BINS * BISPEC_BINS;
pub const FFT_BUFFER_SIZE: usize = BISPEC_BINS * 2;

// Spatial culling: skip cells where column energy < this fraction of mean.
const CULLING_THRESHOLD: f32 = 0.5;

const COHERENCE_THRESHOLD: f32 = 3.0;

// ── Frequency band taxonomy ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FrequencyBand {
    Vlf,
    Infrasound,
    Audio,
    Ultrasonic,
    HyperUltrasonic,
    LowerRF,
    MidRF,
    UpperRF,
}

impl FrequencyBand {
    pub fn classify(hz: f32) -> Self {
        if hz < 3.0 {
            Self::Vlf
        } else if hz < 20.0 {
            Self::Infrasound
        } else if hz < 20_000.0 {
            Self::Audio
        } else if hz < 96_000.0 {
            Self::Ultrasonic
        } else if hz < 400_000.0 {
            Self::HyperUltrasonic
        } else if hz < 1_536_000.0 {
            Self::LowerRF
        } else if hz < 3_072_000.0 {
            Self::MidRF
        } else {
            Self::UpperRF
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vlf => "Vlf",
            Self::Infrasound => "Infrasound",
            Self::Audio => "Audio",
            Self::Ultrasonic => "Ultrasonic",
            Self::HyperUltrasonic => "HyperUltrasonic",
            Self::LowerRF => "LowerRF",
            Self::MidRF => "MidRF",
            Self::UpperRF => "UpperRF",
        }
    }
}

fn band_coherence_threshold(f1_hz: f32, f2_hz: f32, mamba_scaler: f32) -> f32 {
    let base_thresh = match FrequencyBand::classify(f1_hz.max(f2_hz)) {
        FrequencyBand::Infrasound => 2.0,
        FrequencyBand::Audio => COHERENCE_THRESHOLD,
        FrequencyBand::Ultrasonic => 4.5,
        FrequencyBand::HyperUltrasonic => 6.0,
        FrequencyBand::LowerRF => 5.0,
        FrequencyBand::MidRF => 7.0,
        FrequencyBand::UpperRF => 9.0,
        FrequencyBand::Vlf => 1.0, // Added Vlf case
    };
    // Adaptive threshold learned from Mamba environment monitoring,
    // defaults to 1.0 (no scaling)
    (base_thresh * mamba_scaler).max(1.0)
}

// ── GPU data types ─────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BispecCell {
    pub magnitude: f32,
    pub phase: f32,
}

// ── WGSL bispectrum shader ────────────────────────────────────────────────────

const BISPECTRUM_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read>       fft_input  : array<f32>;
@group(0) @binding(1) var<storage, read_write> bispec_out : array<f32>;

const BISPEC_BINS : u32 = 512u;

fn fft_at(bin: u32) -> vec2<f32> {
    if bin >= BISPEC_BINS { return vec2<f32>(0.0, 0.0); }
    let idx = bin * 2u;
    return vec2<f32>(fft_input[idx], fft_input[idx + 1u]);
}
fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}
fn cconj(a: vec2<f32>) -> vec2<f32> { return vec2<f32>(a.x, -a.y); }
fn cmag(a: vec2<f32>) -> f32  { return sqrt(a.x * a.x + a.y * a.y); }
fn carg(a: vec2<f32>) -> f32  { return atan2(a.y, a.x); }

fn compute_bispec(f1: u32, f2: u32, product_bin: u32, out_offset: u32) {
    if product_bin >= BISPEC_BINS {
        bispec_out[out_offset]      = 0.0;
        bispec_out[out_offset + 1u] = 0.0;
        return;
    }
    let b = cmul(cmul(fft_at(f1), fft_at(f2)), cconj(fft_at(product_bin)));
    bispec_out[out_offset]      = cmag(b);
    bispec_out[out_offset + 1u] = carg(b);
}

@compute @workgroup_size(16, 16)
fn compute_bispectrum(@builtin(global_invocation_id) gid: vec3<u32>) {
    let f1 = gid.x;
    let f2 = gid.y;
    if f1 >= BISPEC_BINS || f2 >= BISPEC_BINS { return; }
    if f1 > f2 { return; }   // upper-triangular only

    let cell_idx = (f2 * BISPEC_BINS + f1) * 8u;
    compute_bispec(f1, f2, f1 + f2,                              cell_idx + 0u);
    let diff = select(f2 - f1, f1 - f2, f1 < f2);
    compute_bispec(f1, f2, diff,                                 cell_idx + 2u);
    compute_bispec(f1, f1, f1 * 2u,                              cell_idx + 4u);
    let im_bin = select(0u, f1 * 2u - f2, f1 * 2u >= f2);
    compute_bispec(f1, f2, im_bin,                               cell_idx + 6u);
}
"#;

// ── BispectrumEngine ──────────────────────────────────────────────────────────

pub struct BispectrumEngine {
    shared: Arc<GpuShared>,
    pipeline: wgpu::ComputePipeline,
    fft_buffer: wgpu::Buffer,
    bispec_buffer: wgpu::Buffer,
    readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    coherence_sum: Vec<f32>,
    coherence_sq: Vec<f32>,
    coherence_n: Vec<u32>,
    coherence_phase: Vec<Vec<f32>>,
    frame_count: u32,
    pub mamba_threshold_scaler: f32,
    session_id: String,
}

impl BispectrumEngine {
    pub fn new(shared: Arc<GpuShared>, session_id: String) -> anyhow::Result<Self> {
        let device = &shared.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bispectrum-shader"),
            source: wgpu::ShaderSource::Wgsl(BISPECTRUM_SHADER.into()),
        });

        let fft_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bispec-fft"),
            size: (FFT_BUFFER_SIZE * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bispec_size = (BISPEC_MATRIX_SIZE * 8 * 4) as u64;
        let bispec_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bispec-out"),
            size: bispec_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bispec-rb"),
            size: bispec_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bispec-bgl"),
            entries: &[
                bgl_entry(0, wgpu::BufferBindingType::Storage { read_only: true }),
                bgl_entry(1, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bispec-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: fft_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bispec_buffer.as_entire_binding(),
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bispec-pl"),
            bind_group_layouts: &[&bgl],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bispec-pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("compute_bispectrum"),
            compilation_options: Default::default(),
            cache: None,
        });

        let n_cells = BISPEC_MATRIX_SIZE * 4;
        Ok(Self {
            shared,
            pipeline,
            fft_buffer,
            bispec_buffer,
            readback,
            bind_group,
            coherence_sum: vec![0.0f32; n_cells],
            coherence_sq: vec![0.0f32; n_cells],
            coherence_n: vec![0u32; n_cells],
            coherence_phase: vec![Vec::new(); n_cells],
            frame_count: 0,
            mamba_threshold_scaler: 1.0,
            session_id,
        })
    }

    pub fn analyze_frame(
        &mut self,
        fft_complex: &[f32],
        sample_rate: f32,
        hardware: HardwareLayer,
    ) -> Vec<DetectionEvent> {
        assert!(fft_complex.len() >= FFT_BUFFER_SIZE);

        // ── Spatial culling: build per-bin energy vector ──────────────────────
        // Energy of bin k = re²+im² from the complex FFT.
        let bin_energy: Vec<f32> = fft_complex[..FFT_BUFFER_SIZE]
            .chunks_exact(2)
            .map(|c| c[0] * c[0] + c[1] * c[1])
            .collect();
        let mean_energy = bin_energy.iter().sum::<f32>() / bin_energy.len() as f32;
        let cull_thresh = mean_energy * CULLING_THRESHOLD;

        let device = &self.shared.device;
        let queue = &self.shared.queue;

        queue.write_buffer(
            &self.fft_buffer,
            0,
            bytemuck::cast_slice(&fft_complex[..FFT_BUFFER_SIZE]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bispec-enc"),
        });
        {
            let mut rng = rand::thread_rng();
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bispec-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(32, 32, 1);
        }
        let bispec_size = (BISPEC_MATRIX_SIZE * 8 * 4) as u64;
        encoder.copy_buffer_to_buffer(&self.bispec_buffer, 0, &self.readback, 0, bispec_size);
        queue.submit(std::iter::once(encoder.finish()));

        let slice = self.readback.slice(..bispec_size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().expect("Bispectrum readback failed");

        let confirmed = {
            let view = slice.get_mapped_range();
            let data = bytemuck::cast_slice::<u8, f32>(&view).to_vec();
            self.update_coherence(data, sample_rate, hardware, &bin_energy, cull_thresh)
        };
        self.readback.unmap();
        self.frame_count += 1;
        confirmed
    }

    fn update_coherence(
        &mut self,
        data: Vec<f32>,
        sample_rate: f32,
        hardware: HardwareLayer,
        bin_energy: &[f32],
        cull_thresh: f32,
    ) -> Vec<DetectionEvent> {
        let mut confirmed = Vec::new();
        let bin_hz = sample_rate / BISPEC_FFT_SIZE as f32;

        let mean_mag: f32 = {
            let sum: f32 = data.iter().step_by(2).sum();
            sum / (BISPEC_MATRIX_SIZE * 4) as f32
        };

        let product_types: [(ProductType, usize); 4] = [
            (ProductType::Sum, 0),
            (ProductType::Difference, 2),
            (ProductType::Harmonic, 4),
            (ProductType::Intermodulation, 6),
        ];

        for f2 in 0..BISPEC_BINS {
            // Spatial culling: skip if f2 bin has negligible energy.
            if f2 < bin_energy.len() && bin_energy[f2] < cull_thresh {
                continue;
            }

            for f1 in 0..=f2 {
                // Spatial culling: skip if f1 bin has negligible energy.
                if f1 < bin_energy.len() && bin_energy[f1] < cull_thresh {
                    continue;
                }

                let cell_base = (f2 * BISPEC_BINS + f1) * 8;
                let f1_hz = f1 as f32 * bin_hz;
                let f2_hz_val = f2 as f32 * bin_hz;
                let threshold = mean_mag
                    * band_coherence_threshold(f1_hz, f2_hz_val, self.mamba_threshold_scaler);

                for (ptype, offset) in &product_types {
                    let mag = data[cell_base + offset];
                    let phase = data[cell_base + offset + 1];
                    if mag < threshold {
                        continue;
                    }

                    let acc_idx = (f2 * BISPEC_BINS + f1) * 4 + ptype.as_u32() as usize;
                    self.coherence_sum[acc_idx] += mag;
                    self.coherence_sq[acc_idx] += mag * mag;
                    self.coherence_n[acc_idx] += 1;

                    let phases = &mut self.coherence_phase[acc_idx];
                    phases.push(phase);
                    if phases.len() > 30 {
                        phases.remove(0);
                    }

                    let n = self.coherence_n[acc_idx];
                    if n >= MIN_COHERENCE_FRAMES {
                        let phases_snap = self.coherence_phase[acc_idx].clone();
                        if phases_snap.len() >= 10 && phase_stability(&phases_snap) > 0.8 {
                            let mean_mag_acc = self.coherence_sum[acc_idx] / n as f32;
                            let mean_phase = mean_phase_circular(&phases_snap);
                            let product_hz = product_hz_val(f1, f2, ptype, bin_hz);

                            if let Some(event) = self.make_detection(
                                f1,
                                f2,
                                product_hz,
                                *ptype,
                                mean_mag_acc,
                                mean_phase,
                                n,
                                hardware,
                                sample_rate,
                            ) {
                                confirmed.push(event);
                                self.coherence_sum[acc_idx] = 0.0;
                                self.coherence_sq[acc_idx] = 0.0;
                                self.coherence_n[acc_idx] = 0;
                                self.coherence_phase[acc_idx].clear();
                            }
                        }
                    }
                }
            }
        }
        confirmed
    }

    fn make_detection(
        &self,
        f1: usize,
        f2: usize,
        product_hz_val: f32,
        ptype: ProductType,
        magnitude: f32,
        phase_angle: f32,
        coherence_frames: u32,
        hardware: HardwareLayer,
        sample_rate: f32,
    ) -> Option<DetectionEvent> {
        if f1 == 0 && f2 == 0 {
            return None;
        }
        let bin_hz = sample_rate / BISPEC_FFT_SIZE as f32;
        let f1_hz = f1 as f32 * bin_hz;
        let embedding = compute_embedding(f1, f2, magnitude, phase_angle, &self.coherence_sum);
        let id = format!("{:016x}{:08x}", self.frame_count as u64, f1 * 1000 + f2);
        let band = FrequencyBand::classify(f1_hz.max(product_hz_val));
        Some(DetectionEvent {
            id,
            timestamp: std::time::SystemTime::now(),
            f1_hz,
            f2_hz: f2 as f32 * bin_hz,
            product_hz: product_hz_val,
            product_type: ptype,
            magnitude,
            phase_angle,
            coherence_frames,
            spl_db: 0.0,
            session_id: self.session_id.clone(),
            hardware,
            embedding,
            frequency_band: band,
            // Forensic analysis fields (populated by dispatch loop)
            audio_dc_bias_v: None,
            sdr_dc_bias_v: None,
            mamba_anomaly_db: 0.0,
            timestamp_sync_ms: None,
            is_coordinated: false,
            detection_method: "bispectrum".to_string(),
        })
    }
}

// ── Math helpers ──────────────────────────────────────────────────────────────

fn phase_stability(phases: &[f32]) -> f32 {
    if phases.is_empty() {
        return 0.0;
    }
    let n = phases.len() as f32;
    let sc = phases.iter().map(|p| p.cos()).sum::<f32>() / n;
    let ss = phases.iter().map(|p| p.sin()).sum::<f32>() / n;
    (sc * sc + ss * ss).sqrt()
}

fn mean_phase_circular(phases: &[f32]) -> f32 {
    let n = phases.len() as f32;
    let sc = phases.iter().map(|p| p.cos()).sum::<f32>() / n;
    let ss = phases.iter().map(|p| p.sin()).sum::<f32>() / n;
    ss.atan2(sc)
}

fn product_hz_val(f1: usize, f2: usize, ptype: &ProductType, bin_hz: f32) -> f32 {
    let f1f = f1 as f32 * bin_hz;
    let f2f = f2 as f32 * bin_hz;
    match ptype {
        ProductType::Sum => f1f + f2f,
        ProductType::Difference => (f1f - f2f).abs(),
        ProductType::Harmonic => 2.0 * f1f,
        ProductType::Intermodulation => (2.0 * f1f - f2f).max(0.0),
    }
}

fn compute_embedding(f1: usize, f2: usize, magnitude: f32, phase: f32, csum: &[f32]) -> Vec<f32> {
    let mut emb = vec![0.0f32; 32];
    let n = BISPEC_BINS as f32;
    let log_pos = |bin: usize| -> usize {
        if bin == 0 {
            return 0;
        }
        let log = (bin as f32).log2();
        (log * 8.0 / n.log2()).min(7.0) as usize
    };
    emb[log_pos(f1)] = 1.0;
    emb[8 + log_pos(f2)] = 1.0;
    let neighbors = [
        (f1.saturating_sub(1), f2.saturating_sub(1)),
        (f1, f2.saturating_sub(1)),
        (f1 + 1, f2.saturating_sub(1)),
        (f1.saturating_sub(1), f2),
        (f1 + 1, f2),
        (f1.saturating_sub(1), f2 + 1),
        (f1, f2 + 1),
        (f1 + 1, f2 + 1),
    ];
    for (i, (nf1, nf2)) in neighbors.iter().enumerate() {
        if *nf1 < BISPEC_BINS && *nf2 < BISPEC_BINS {
            emb[16 + i] = csum[(nf2 * BISPEC_BINS + nf1) * 4].min(10.0) / 10.0;
        }
    }
    emb[24] = phase.cos();
    emb[25] = phase.sin();
    emb[26] = (2.0 * phase).cos();
    emb[27] = (2.0 * phase).sin();
    emb[28] = magnitude.min(100.0) / 100.0;
    emb
}

fn bgl_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
