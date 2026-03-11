use crate::dispatch::signal_ingester::{SignalIngester, SignalMetadata, SampleFormat, SignalType};
use crate::ml::field_particle::FieldParticle;
use crate::ml::modular_features::VideoFrame;

/// Visual microphone ingester for C925e webcam
/// Converts RGB/YUV pixel data -> FieldParticle stream with visual features
pub struct VisualIngester {
    frame_width: u32,
    frame_height: u32,
}

impl VisualIngester {
    pub fn new(frame_width: u32, frame_height: u32) -> Self {
        Self {
            frame_width,
            frame_height,
        }
    }

    /// Extract frequency bin energy from pixel intensity variations
    /// Returns energy in low/mid/high bins (or detailed bins if configured)
    fn extract_frequency_bins(&self, frame_current: &[u8], frame_prev: Option<&[u8]>, num_bins: usize) -> Vec<f32> {
        let mut bins = vec![0.0f32; num_bins];
        
        if frame_current.is_empty() {
            return bins;
        }

        // Compute per-pixel intensity change (temporal derivative)
        let total_pixels = (self.frame_width * self.frame_height) as usize;
        let mut intensity_changes = Vec::with_capacity(total_pixels);
        
        for i in 0..total_pixels {
            let curr_intensity = if i * 3 < frame_current.len() {
                // RGB to luminance: Y = 0.299*R + 0.587*G + 0.114*B
                let r = frame_current[i * 3] as f32;
                let g = if i * 3 + 1 < frame_current.len() { frame_current[i * 3 + 1] as f32 } else { 0.0 };
                let b = if i * 3 + 2 < frame_current.len() { frame_current[i * 3 + 2] as f32 } else { 0.0 };
                0.299 * r + 0.587 * g + 0.114 * b
            } else {
                0.0
            };

            let prev_intensity = if let Some(prev) = frame_prev {
                if i * 3 < prev.len() {
                    let r = prev[i * 3] as f32;
                    let g = if i * 3 + 1 < prev.len() { prev[i * 3 + 1] as f32 } else { 0.0 };
                    let b = if i * 3 + 2 < prev.len() { prev[i * 3 + 2] as f32 } else { 0.0 };
                    0.299 * r + 0.587 * g + 0.114 * b
                } else {
                    0.0
                }
            } else {
                curr_intensity
            };

            intensity_changes.push((curr_intensity - prev_intensity).abs());
        }

        // Bin the intensity changes by magnitude (simulating frequency response)
        let max_change = intensity_changes.iter().cloned().fold(0.0f32, f32::max);
        if max_change > 0.0 {
            for &change in &intensity_changes {
                let bin_idx = ((change / max_change) * num_bins as f32) as usize;
                if bin_idx < num_bins {
                    bins[bin_idx] += change;
                }
            }
        }

        // Normalize
        let max_bin = bins.iter().cloned().fold(0.0f32, f32::max);
        if max_bin > 0.0 {
            for bin in &mut bins {
                *bin /= max_bin;
            }
        }

        bins
    }

    /// Extract optical flow features (simplified block matching)
    /// Returns 12-D flow vector: [dx_mean, dy_mean, dx_var, dy_var, flow_magnitude, ...]
    fn extract_optical_flow(&self, frame_current: &[u8], frame_prev: Option<&[u8]>) -> [f32; 12] {
        let mut flow = [0.0f32; 12];
        
        if frame_prev.is_none() || frame_current.len() != frame_prev.unwrap().len() {
            return flow;
        }

        let prev = frame_prev.unwrap();
        let total_pixels = (self.frame_width * self.frame_height) as usize;
        
        let mut sum_dx = 0.0f32;
        let mut sum_dy = 0.0f32;
        let mut sum_dx_sq = 0.0f32;
        let mut sum_dy_sq = 0.0f32;
        let mut flow_count = 0;

        // Block-based flow estimation (8x8 blocks)
        let block_size = 8;
        for by in 0..(self.frame_height as usize / block_size) {
            for bx in 0..(self.frame_width as usize / block_size) {
                let block_idx = (by * block_size * self.frame_width as usize + bx * block_size) * 3;
                let prev_idx = block_idx;
                
                if block_idx + 3 < frame_current.len() && prev_idx + 3 < prev.len() {
                    // Simplified: use intensity gradient for flow direction
                    let dx = (frame_current[block_idx + 3] as f32 - prev[prev_idx] as f32).abs();
                    let dy = if block_idx + self.frame_width as usize * 3 < frame_current.len() {
                        (frame_current[block_idx + self.frame_width as usize * 3] as f32 - prev[prev_idx] as f32).abs()
                    } else {
                        0.0
                    };

                    sum_dx += dx;
                    sum_dy += dy;
                    sum_dx_sq += dx * dx;
                    sum_dy_sq += dy * dy;
                    flow_count += 1;
                }
            }
        }

        if flow_count > 0 {
            let mean_dx = sum_dx / flow_count as f32;
            let mean_dy = sum_dy / flow_count as f32;
            let var_dx = (sum_dx_sq / flow_count as f32) - (mean_dx * mean_dx);
            let var_dy = (sum_dy_sq / flow_count as f32) - (mean_dy * mean_dy);
            let magnitude = (mean_dx * mean_dx + mean_dy * mean_dy).sqrt();

            flow[0] = mean_dx;
            flow[1] = mean_dy;
            flow[2] = var_dx.max(0.0);
            flow[3] = var_dy.max(0.0);
            flow[4] = magnitude;
            flow[5] = flow_count as f32 / (self.frame_width as f32 * self.frame_height as f32); // Flow density
        }

        flow
    }

    /// Extract spatial coherence (correlation between regions)
    fn extract_coherence(&self, frame: &[u8]) -> [f32; 3] {
        let mut coherence = [0.0f32; 3];
        
        if frame.is_empty() {
            return coherence;
        }

        // Divide frame into 3 regions (left, center, right)
        let region_width = self.frame_width / 3;
        let total_pixels = (self.frame_width * self.frame_height) as usize;
        
        let mut region_means = [0.0f32; 3];
        let mut region_counts = [0usize; 3];

        for i in 0..(total_pixels / 3) {
            let idx = i * 3;
            if idx + 2 < frame.len() {
                let lum = 0.299 * frame[idx] as f32 + 0.587 * frame[idx + 1] as f32 + 0.114 * frame[idx + 2] as f32;
                region_means[0] += lum;
                region_counts[0] += 1;
            }
        }

        for i in (total_pixels / 3)..(2 * total_pixels / 3) {
            let idx = i * 3;
            if idx + 2 < frame.len() {
                let lum = 0.299 * frame[idx] as f32 + 0.587 * frame[idx + 1] as f32 + 0.114 * frame[idx + 2] as f32;
                region_means[1] += lum;
                region_counts[1] += 1;
            }
        }

        for i in (2 * total_pixels / 3)..total_pixels {
            let idx = i * 3;
            if idx + 2 < frame.len() {
                let lum = 0.299 * frame[idx] as f32 + 0.587 * frame[idx + 1] as f32 + 0.114 * frame[idx + 2] as f32;
                region_means[2] += lum;
                region_counts[2] += 1;
            }
        }

        for i in 0..3 {
            if region_counts[i] > 0 {
                region_means[i] /= region_counts[i] as f32;
            }
        }

        // Coherence = inverse of variance between regions
        let overall_mean = (region_means[0] + region_means[1] + region_means[2]) / 3.0;
        let variance = region_means.iter().map(|&m| (m - overall_mean).powi(2)).sum::<f32>() / 3.0;
        coherence[0] = 1.0 / (1.0 + variance); // Higher = more coherent

        coherence[1] = (region_means[0] - region_means[1]).abs() / 255.0; // Left-center diff
        coherence[2] = (region_means[1] - region_means[2]).abs() / 255.0; // Center-right diff

        coherence
    }

    /// Extract global visual features
    fn extract_global_features(&self, frame: &[u8]) -> [f32; 4] {
        let mut global = [0.0f32; 4];
        
        if frame.is_empty() {
            return global;
        }

        let mut sum_lum = 0.0f32;
        let mut sum_sat = 0.0f32;
        let mut pixel_count = 0;

        for i in 0..(frame.len() / 3) {
            let r = frame[i * 3] as f32 / 255.0;
            let g = if i * 3 + 1 < frame.len() { frame[i * 3 + 1] as f32 / 255.0 } else { 0.0 };
            let b = if i * 3 + 2 < frame.len() { frame[i * 3 + 2] as f32 / 255.0 } else { 0.0 };

            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            let max_rgb = r.max(g).max(b);
            let min_rgb = r.min(g).min(b);
            let sat = if max_rgb > 0.0 { (max_rgb - min_rgb) / max_rgb } else { 0.0 };

            sum_lum += lum;
            sum_sat += sat;
            pixel_count += 1;
        }

        if pixel_count > 0 {
            global[0] = sum_lum / pixel_count as f32; // Mean luminance
            global[1] = sum_sat / pixel_count as f32; // Mean saturation
            global[2] = frame.len() as f32 / (self.frame_width as f32 * self.frame_height as f32 * 3.0); // Coverage
        }

        global[3] = self.frame_width as f32 / self.frame_height as f32; // Aspect ratio

        global
    }

    /// Extract color features (RGB distribution)
    fn extract_color_features(&self, frame: &[u8]) -> [f32; 4] {
        let mut color = [0.0f32; 4];
        
        if frame.is_empty() {
            return color;
        }

        let mut sum_r = 0.0f32;
        let mut sum_g = 0.0f32;
        let mut sum_b = 0.0f32;
        let mut pixel_count = 0;

        for i in 0..(frame.len() / 3) {
            sum_r += frame[i * 3] as f32 / 255.0;
            if i * 3 + 1 < frame.len() {
                sum_g += frame[i * 3 + 1] as f32 / 255.0;
            }
            if i * 3 + 2 < frame.len() {
                sum_b += frame[i * 3 + 2] as f32 / 255.0;
            }
            pixel_count += 1;
        }

        if pixel_count > 0 {
            color[0] = sum_r / pixel_count as f32;
            color[1] = sum_g / pixel_count as f32;
            color[2] = sum_b / pixel_count as f32;
            color[3] = (color[0] + color[1] + color[2]) / 3.0; // Mean color intensity
        }

        color
    }
}

impl SignalIngester for VisualIngester {
    fn ingest(
        &self,
        raw_signal: &[u8],
        timestamp_us: u64,
        metadata: &SignalMetadata,
    ) -> Vec<FieldParticle> {
        let mut particles = Vec::new();

        // Extract visual features based on metadata configuration
        let num_bins = metadata.num_channels as usize; // Reuse num_channels for frequency bins
        let preserve_rgb = metadata.carrier_freq_hz.map_or(false, |f| f > 0.0); // Reuse carrier_freq as flag

        // Extract feature components
        let frequency_bins = self.extract_frequency_bins(raw_signal, None, num_bins);
        let flow = self.extract_optical_flow(raw_signal, None);
        let coherence = self.extract_coherence(raw_signal);
        let global = self.extract_global_features(raw_signal);
        let color = self.extract_color_features(raw_signal);

        // Build visual feature vector
        let mut visual_features = Vec::new();
        
        if preserve_rgb {
            // RGB separation mode: (3 * bins) + 12 + 3 + 4 + 4
            // Expand bins for R, G, B channels
            for i in 0..num_bins {
                visual_features.push(frequency_bins[i % frequency_bins.len()] * color[0]); // R-weighted
            }
            for i in 0..num_bins {
                visual_features.push(frequency_bins[i % frequency_bins.len()] * color[1]); // G-weighted
            }
            for i in 0..num_bins {
                visual_features.push(frequency_bins[i % frequency_bins.len()] * color[2]); // B-weighted
            }
        } else {
            // Luminance mode: bins + 4 + 4
            visual_features.extend_from_slice(&frequency_bins);
        }

        visual_features.extend_from_slice(&flow);
        visual_features.extend_from_slice(&coherence);
        visual_features.extend_from_slice(&global);
        
        if preserve_rgb {
            visual_features.extend_from_slice(&color);
        }

        // Convert visual features to particles
        // Each feature dimension becomes a particle with spatial encoding
        for (i, &feature) in visual_features.iter().enumerate() {
            let x = (i % self.frame_width as usize) as f32 / self.frame_width as f32;
            let y = (i / self.frame_width as usize) as f32 / self.frame_height as f32;
            
            particles.push(FieldParticle {
                position: [x, y, timestamp_us as f32 / 1e6], // Normalized spatial + temporal
                phase_i: feature,
                phase_q: 0.0, // Visual features are real-valued
                energy: feature.abs(),
                material_id: 0x0200, // Visual microphone cluster
                _padding: [0; 3],
            });
        }

        particles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_ingester_creation() {
        let ingester = VisualIngester::new(640, 480);
        assert_eq!(ingester.frame_width, 640);
        assert_eq!(ingester.frame_height, 480);
    }

    #[test]
    fn test_visual_ingester_empty_frame() {
        let ingester = VisualIngester::new(640, 480);
        let metadata = SignalMetadata {
            signal_type: SignalType::Video,
            sample_rate_hz: 30,
            carrier_freq_hz: Some(1.0), // RGB mode
            num_channels: 3,
            sample_format: SampleFormat::I16,
        };

        let particles = ingester.ingest(&[], 0, &metadata);
        assert!(particles.is_empty());
    }

    #[test]
    fn test_visual_ingester_basic() {
        let ingester = VisualIngester::new(4, 4);
        let metadata = SignalMetadata {
            signal_type: SignalType::Video,
            sample_rate_hz: 30,
            carrier_freq_hz: Some(0.0), // Luminance mode
            num_channels: 3,
            sample_format: SampleFormat::I16,
        };

        // Create a simple test frame (4x4 RGB)
        let frame = vec![128u8; 4 * 4 * 3];
        let particles = ingester.ingest(&frame, 1000, &metadata);
        
        assert!(!particles.is_empty());
        for particle in &particles {
            assert!(particle.position[0] >= 0.0 && particle.position[0] <= 1.0);
            assert!(particle.position[1] >= 0.0 && particle.position[1] <= 1.0);
            assert_eq!(particle.material_id, 0x0200);
        }
    }

    #[test]
    fn test_frequency_bins() {
        let ingester = VisualIngester::new(8, 8);
        let frame = vec![255u8; 8 * 8 * 3];
        let bins = ingester.extract_frequency_bins(&frame, None, 3);
        
        assert_eq!(bins.len(), 3);
        // All pixels identical, so no change -> all bins should be 0
        for &bin in &bins {
            assert_eq!(bin, 0.0);
        }
    }

    #[test]
    fn test_optical_flow() {
        let ingester = VisualIngester::new(16, 16);
        let frame = vec![100u8; 16 * 16 * 3];
        let flow = ingester.extract_optical_flow(&frame, Some(&frame));
        
        assert_eq!(flow.len(), 12);
        // Identical frames -> no flow
        assert_eq!(flow[0], 0.0); // mean_dx
        assert_eq!(flow[1], 0.0); // mean_dy
    }

    #[test]
    fn test_coherence() {
        let ingester = VisualIngester::new(12, 8);
        let frame = vec![128u8; 12 * 8 * 3];
        let coherence = ingester.extract_coherence(&frame);
        
        assert_eq!(coherence.len(), 3);
        // Uniform frame -> high coherence
        assert!(coherence[0] > 0.5);
    }

    #[test]
    fn test_global_features() {
        let ingester = VisualIngester::new(8, 8);
        let frame = vec![255u8; 8 * 8 * 3];
        let global = ingester.extract_global_features(&frame);
        
        assert_eq!(global.len(), 4);
        assert!(global[0] > 0.9); // High luminance for white frame
        assert!(global[2] > 0.0); // Coverage
    }

    #[test]
    fn test_color_features() {
        let ingester = VisualIngester::new(8, 8);
        let mut frame = vec![0u8; 8 * 8 * 3]; // RGB888
        for px in frame.chunks_exact_mut(3) {
            px[0] = 255; // R
        }
        let color = ingester.extract_color_features(&frame);
        
        assert_eq!(color.len(), 4);
        assert!(color[0] > 0.9); // High red
        assert!(color[1] < 0.1); // Low green
        assert!(color[2] < 0.1); // Low blue
    }
}

