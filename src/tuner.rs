//! RTL-SDR Radio Tuner TUI
//!
//! A simple radio tuner interface from 10 kHz to 300 MHz
//! with audio demodulation output.
//!
//! Usage:
//! ```bash
//! cargo run --bin tuner
//! ```

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use std::{io, time::Duration, sync::{Arc, Mutex}};
use crate::hardware::rtlsdr::{RtlSdrConfig, RtlSdrDevice};
use num_complex::Complex32;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const MIN_FREQ: f64 = 10_000.0;      // 10 kHz
const MAX_FREQ: f64 = 300_000_000.0; // 300 MHz
const STEP_FINE: f64 = 100.0;        // 100 Hz fine tune
const STEP_COARSE: f64 = 1_000_000.0; // 1 MHz coarse tune

#[derive(Debug, Clone, Copy, PartialEq)]
enum Modulation {
    AM,
    NFM,
    WFM,
    LSB,
    USB,
}

impl Modulation {
    fn as_str(&self) -> &'static str {
        match self {
            Modulation::AM => "AM",
            Modulation::NFM => "NFM",
            Modulation::WFM => "WFM",
            Modulation::LSB => "LSB",
            Modulation::USB => "USB",
        }
    }

    fn bandwidth(&self) -> u32 {
        match self {
            Modulation::AM => 10_000,
            Modulation::NFM => 12_500,
            Modulation::WFM => 200_000,
            Modulation::LSB | Modulation::USB => 3_000,
        }
    }
}

struct TunerApp {
    device: RtlSdrDevice,
    frequency: f64,
    modulation: Modulation,
    gain_db: f32,
    signal_strength: f32,
    running: bool,
    audio_gain: f32,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    _audio_stream: Option<cpal::Stream>,
    sim_phase: f32,
}

impl TunerApp {
    fn new() -> Result<Self> {
        let config = RtlSdrConfig::default();
        let rtl_device = RtlSdrDevice::new(config)?;
        
        // Print device info
        if rtl_device.is_available() {
            if let Some(info) = rtl_device.get_device_info() {
                eprintln!("RTL-SDR Device: {}", info);
            } else {
                eprintln!("RTL-SDR Device: Connected");
            }
        } else {
            eprintln!("WARNING: No RTL-SDR device detected. Running in simulation mode.");
        }

        // Setup audio output with shared buffer
        let audio_buffer = Arc::new(Mutex::new(Vec::with_capacity(4096)));
        let audio_buffer_clone = Arc::clone(&audio_buffer);

        let host = cpal::default_host();
        let audio_device = host.default_output_device().ok_or_else(|| anyhow::anyhow!("No audio output device"))?;
        let config = audio_device.default_output_config()?;
        
        eprintln!("Audio Output: {}", audio_device.name().unwrap_or_else(|_| "Unknown".to_string()));

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let config = config.config();
                audio_device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if let Ok(mut buf) = audio_buffer_clone.lock() {
                            for sample in data.iter_mut() {
                                *sample = buf.pop().unwrap_or(0.0);
                            }
                        } else {
                            data.fill(0.0);
                        }
                    },
                    |_err| eprintln!("Audio error: {}", _err),
                    None,
                )?
            }
            _ => anyhow::bail!("Unsupported sample format"),
        };
        stream.play()?;

        Ok(Self {
            device: rtl_device,
            frequency: 101_100_000.0, // Start at 101.1 MHz FM
            modulation: Modulation::WFM,
            gain_db: 30.0,
            signal_strength: 0.0,
            running: true,
            audio_gain: 1.0,
            audio_buffer,
            _audio_stream: Some(stream),
            sim_phase: 0.0,
        })
    }

    fn tune(&mut self, delta: f64) {
        self.frequency = (self.frequency + delta).clamp(MIN_FREQ, MAX_FREQ);
        let _ = self.device.set_frequency(self.frequency);
    }

    fn set_modulation(&mut self, mod_type: Modulation) {
        self.modulation = mod_type;
        let _ = self.device.set_gain(self.gain_db);
    }

    fn adjust_gain(&mut self, delta: f32) {
        self.gain_db = (self.gain_db + delta).clamp(0.0, 50.0);
        let _ = self.device.set_gain(self.gain_db);
    }

    fn update_signal_strength(&mut self) {
        // Capture and demodulate samples
        if let Ok(iq_samples) = self.device.capture(4096) {
            // If no device, generate simulated signal
            let samples = if iq_samples.is_empty() || !self.device.is_available() {
                // Generate simulated FM signal with noise
                self.generate_simulated_signal(4096)
            } else {
                iq_samples
            };

            if samples.is_empty() {
                self.signal_strength = 0.0;
                return;
            }

            // Calculate signal strength (power)
            let power: f32 = samples.iter()
                .map(|s| s.norm_sqr())
                .sum::<f32>() / samples.len() as f32;
            self.signal_strength = (power.sqrt() * 2.0).min(1.0);

            // Demodulate based on modulation type
            let audio = self.demodulate(&samples);

            // Push to audio buffer (insert at beginning for FIFO)
            if let Ok(mut buf) = self.audio_buffer.lock() {
                for sample in audio.into_iter().rev() {
                    buf.push(sample * self.audio_gain);
                }
            }
        }
    }

    /// Generate simulated RF signal for testing without hardware
    fn generate_simulated_signal(&mut self, num_samples: usize) -> Vec<Complex32> {
        use rand::Rng;
        let mut rng = rand::rng();
        
        // Generate a test tone (1kHz sine wave) modulated onto "carrier"
        let sample_rate = 2_048_000f32; // 2.048 Msps
        let tone_freq = 1000.0f32; // 1 kHz test tone
        let modulation_index = 0.5f32; // FM modulation index
        
        let mut samples = Vec::with_capacity(num_samples);
        let phase_increment = 2.0 * std::f32::consts::PI * tone_freq / sample_rate;
        
        for _ in 0..num_samples {
            // FM modulation: phase = carrier + modulation_index * sin(tone)
            let modulation = modulation_index * self.sim_phase.sin();
            let phase = modulation;
            
            // Add some noise
            let noise_i: f32 = rng.random_range(-0.1..0.1);
            let noise_q: f32 = rng.random_range(-0.1..0.1);
            
            let i = phase.cos() + noise_i;
            let q = phase.sin() + noise_q;
            
            samples.push(Complex32::new(i, q));
            self.sim_phase += phase_increment;
            if self.sim_phase > 2.0 * std::f32::consts::PI {
                self.sim_phase -= 2.0 * std::f32::consts::PI;
            }
        }
        
        samples
    }

    fn demodulate(&self, iq_samples: &[Complex32]) -> Vec<f32> {
        match self.modulation {
            Modulation::AM => {
                // AM demodulation: envelope detection
                iq_samples.iter()
                    .map(|s| s.norm() * self.audio_gain)
                    .collect()
            }
            Modulation::NFM | Modulation::WFM => {
                // FM demodulation: phase discriminator
                let mut audio = Vec::with_capacity(iq_samples.len() - 1);
                for i in 1..iq_samples.len() {
                    let phase_diff = iq_samples[i].arg() - iq_samples[i - 1].arg();
                    // Normalize phase to [-pi, pi]
                    let normalized = if phase_diff > std::f32::consts::PI {
                        phase_diff - 2.0 * std::f32::consts::PI
                    } else if phase_diff < -std::f32::consts::PI {
                        phase_diff + 2.0 * std::f32::consts::PI
                    } else {
                        phase_diff
                    };
                    audio.push(normalized / std::f32::consts::PI * self.audio_gain);
                }
                audio
            }
            Modulation::LSB | Modulation::USB => {
                // SSB demodulation: product detector (simplified)
                // For proper SSB, we'd need a BFO oscillator
                iq_samples.iter()
                    .map(|s| s.re * self.audio_gain)
                    .collect()
            }
        }
    }
}

pub fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = TunerApp::new()?;

    // Run UI loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TunerApp,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.running = false,
                    KeyCode::Right | KeyCode::Char('l') => app.tune(STEP_FINE),
                    KeyCode::Left | KeyCode::Char('h') => app.tune(-STEP_FINE),
                    KeyCode::Char('L') | KeyCode::Char('H') => {
                        if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                            app.tune(STEP_COARSE)
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.adjust_gain(1.0),
                    KeyCode::Down | KeyCode::Char('j') => app.adjust_gain(-1.0),
                    KeyCode::Char('1') => app.set_modulation(Modulation::AM),
                    KeyCode::Char('2') => app.set_modulation(Modulation::NFM),
                    KeyCode::Char('3') => app.set_modulation(Modulation::WFM),
                    KeyCode::Char('4') => app.set_modulation(Modulation::LSB),
                    KeyCode::Char('5') => app.set_modulation(Modulation::USB),
                    KeyCode::Char('a') => app.audio_gain = (app.audio_gain + 0.1).min(3.0),
                    KeyCode::Char('z') => app.audio_gain = (app.audio_gain - 0.1).max(0.0),
                    KeyCode::Char('r') => {
                        // Rescan / refresh
                        if let Some(info) = app.device.get_device_info() {
                            log::info!("Device: {}", info);
                        }
                    }
                    _ => {}
                }
            }
        }

        app.update_signal_strength();

        if !app.running {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &TunerApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Frequency display
            Constraint::Length(3),  // Signal strength
            Constraint::Length(10), // Modulation & controls
            Constraint::Min(0),     // Help
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("📻 RTL-SDR Radio Tuner")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Frequency display
    let freq_mhz = app.frequency / 1_000_000.0;
    let freq_str = if app.frequency >= 1_000_000.0 {
        format!("{:.3} MHz", freq_mhz)
    } else if app.frequency >= 1_000.0 {
        format!("{:.3} kHz", app.frequency / 1_000.0)
    } else {
        format!("{:.0} Hz", app.frequency)
    };

    let freq_display = Paragraph::new(Line::from(vec![
        Span::raw("Frequency: "),
        Span::styled(
            freq_str,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Tuning"));
    f.render_widget(freq_display, chunks[1]);

    // Signal strength
    let signal_pct = app.signal_strength;
    let signal_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green))
        .percent((signal_pct * 100.0) as u16)
        .label(format!("{:.1}%", signal_pct * 100.0))
        .block(Block::default().borders(Borders::ALL).title("Signal"));
    f.render_widget(signal_gauge, chunks[2]);

    // Modulation and controls
    let device_info = if app.device.is_available() {
        app.device
            .get_device_info()
            .unwrap_or_else(|| "RTL-SDR connected".to_string())
    } else {
        "No device".to_string()
    };

    let mod_lines = vec![
        Line::from(format!(
            "Modulation: {} (BW: {} kHz)",
            app.modulation.as_str(),
            app.modulation.bandwidth() / 1_000
        )),
        Line::from(format!("Gain: {:.1} dB", app.gain_db)),
        Line::from(format!("Audio Gain: {:.1}x", app.audio_gain)),
        Line::from(format!("Device: {}", device_info)),
    ];

    let mod_widget = Paragraph::new(mod_lines)
        .block(Block::default().borders(Borders::ALL).title("Settings"));
    f.render_widget(mod_widget, chunks[3]);

    // Help
    let help_text = vec![
        Line::from("Controls:"),
        Line::from("  ←/→ or h/l : Fine tune (±100 Hz)"),
        Line::from("  H/L (Shift): Coarse tune (±1 MHz)"),
        Line::from("  ↑/↓ or k/j : Adjust gain"),
        Line::from("  1-5        : AM/NFM/WFM/LSB/USB"),
        Line::from("  a/z        : Audio gain up/down"),
        Line::from("  r          : Refresh device"),
        Line::from("  q          : Quit"),
    ];

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[4]);
}
