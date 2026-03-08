use rustfft::num_complex::Complex;
use twister::rtlsdr::RtlSdrEngine;

const SDR_SAMPLE_RATE: u32 = 1_200_000;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() -> anyhow::Result<()> {
    println!("=== Twister: RTL-SDR WFM Radio Demo ===");

    // Default to a common FM station (adjust as needed for your area)
    let freq_mhz: f32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(99.1); // MHz

    let center_freq_hz = (freq_mhz * 1_000_000.0) as u32;

    println!("Tuning to {} MHz...", freq_mhz);

    let mut engine = RtlSdrEngine::new()?;
    engine.set_sample_rate(SDR_SAMPLE_RATE)?;
    engine.tune(center_freq_hz)?;
    engine.set_agc_mode(true)?;

    println!("SDR Started. Sample rate: {} Hz", SDR_SAMPLE_RATE);

    // Audio setup
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No output device available"))?;

    #[allow(deprecated)]
    let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
    println!("Audio output: {}", device_name);

    let config = device.default_output_config()?;
    let sample_format = config.sample_format();
    let channels = config.channels() as usize;

    // Use a shared ring buffer for audio
    let audio_sample_rate = config.sample_rate();
    println!("Device target audio rate: {} Hz", audio_sample_rate);
    let decimation_factor = (SDR_SAMPLE_RATE / audio_sample_rate) as usize;

    let (tx, rx) = crossbeam_channel::bounded::<f32>(audio_sample_rate as usize * 2);

    let audio_config: cpal::StreamConfig = config.into();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &audio_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let value = rx.try_recv().unwrap_or(0.0);
                    for sample in frame.iter_mut() {
                        *sample = value;
                    }
                }
            },
            err_fn,
            None,
        )?,
        _ => return Err(anyhow::anyhow!("Unsupported audio format. Requires F32.")),
    };

    stream.play()?;

    // Demodulation state
    let mut prev_complex = Complex::new(0.0f32, 0.0f32);

    // Simple moving average filter for downsampling
    let mut sma_buffer = vec![0.0f32; decimation_factor];
    let mut sma_idx = 0;

    // De-emphasis filter state
    let mut deemph_state = 0.0f32;
    // alpha = dt / (RC + dt) where RC = 75us for US FM (50us for EU)
    let rc = 75.0e-6;
    let dt = 1.0 / audio_sample_rate as f32;
    let alpha = dt / (rc + dt);

    println!("Playing... Press Ctrl+C to exit.");

    loop {
        let iq: Vec<Complex<f32>> = match engine.read_iq() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("SDR Read Error: {:?}", e);
                break;
            }
        };

        for sample in iq {
            // FM Demodulation using polar discriminator
            // Phase diff = angle(current * conj(prev))
            let conj_prev = Complex::new(prev_complex.re, -prev_complex.im);
            let ds = sample * conj_prev;
            let fm_demo = ds.im.atan2(ds.re); // radians between -PI and PI

            // Applying moving average filter for decimation
            if decimation_factor > 0 {
                sma_buffer[sma_idx] = fm_demo;
                sma_idx += 1;

                if sma_idx >= decimation_factor {
                    sma_idx = 0;
                    let sum: f32 = sma_buffer.iter().sum();
                    let decimated = sum / decimation_factor as f32;

                    // De-emphasis filter (simple IIR low-pass)
                    deemph_state = deemph_state + alpha * (decimated - deemph_state);

                    // Volume scaling (increased to 8.0 to hear quiet stations better)
                    let mut audio_out = deemph_state * 8.0;

                    // Clamp
                    audio_out = audio_out.clamp(-1.0, 1.0);

                    // Ignore if buffer is full rather than blocking forever
                    let _ = tx.try_send(audio_out);
                }
            } else {
                // No decimation needed (shouldn't happen with 1.2M vs 48k)
                let _ = tx.try_send(fm_demo.clamp(-1.0, 1.0));
            }

            prev_complex = sample;
        }
    }

    Ok(())
}
