use slint::{Image, SharedPixelBuffer, Rgba8Pixel};
use std::fmt::Write;
use std::time::Duration;
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem}};

slint::include_modules!();

// The "Forensic Gradient" LERP function
fn lerp_forensic_gradient(val: f32) -> [u8; 4] {
    let c1 = [5, 5, 5, 255]; // #050505
    let c2 = [0, 34, 68, 255]; // #002244
    let c3 = [0, 204, 255, 255]; // #00ccff
    let c4 = [187, 0, 255, 255]; // #bb00ff

    let clamp_val = val.clamp(0.0, 1.0);

    let (t, start, end) = if clamp_val <= 0.2 {
        (clamp_val / 0.2, c1, c2)
    } else if clamp_val <= 0.6 {
        ((clamp_val - 0.2) / 0.4, c2, c3)
    } else {
        ((clamp_val - 0.6) / 0.4, c3, c4)
    };

    let r = (start[0] as f32 * (1.0 - t) + end[0] as f32 * t) as u8;
    let g = (start[1] as f32 * (1.0 - t) + end[1] as f32 * t) as u8;
    let b = (start[2] as f32 * (1.0 - t) + end[2] as f32 * t) as u8;
    let a = 255;

    [r, g, b, a]
}

pub struct WaterfallBuffer {
    buffer: SharedPixelBuffer<Rgba8Pixel>,
    width: u32,
    height: u32,
}

impl WaterfallBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: SharedPixelBuffer::new(width, height),
            width,
            height,
        }
    }

    pub fn push_new_line(&mut self, fft_data: &[f32]) {
        let width = self.width;
        let height = self.height;
        let raw_pixels = self.buffer.make_mut_bytes();

        let row_stride = (width * 4) as usize;
        raw_pixels.copy_within(0..(row_stride * (height - 1) as usize), row_stride);

        for x in 0..width {
            let val = fft_data.get(x as usize).copied().unwrap_or(0.0);
            let color = lerp_forensic_gradient(val);
            let idx = (x * 4) as usize;
            raw_pixels[idx..idx+4].copy_from_slice(&color);
        }
    }

    pub fn to_slint_image(&self) -> Image {
        Image::from_rgba8(self.buffer.clone())
    }
}

use noise::{NoiseFn, Perlin};

fn generate_mock_fft(t: f32, num_bins: usize) -> Vec<f32> {
    let perlin = Perlin::new(42);
    let mut fft = vec![0.0; num_bins];

    let peak1_pos = (num_bins as f32 * (0.3 + 0.2 * (t * 0.5).sin())) as usize;
    let noise_val = perlin.get([t as f64 * 0.2, 0.0]) as f32;
    let peak2_pos = (num_bins as f32 * (0.6 + 0.15 * noise_val)) as usize;
    let peak3_pos = (num_bins as f32 * 0.8) as usize;

    for i in 0..num_bins {
        let mut val = 0.0;

        val += 0.05 * (perlin.get([i as f64 * 0.1, t as f64]) as f32).abs();

        let dist1 = (i as f32 - peak1_pos as f32).abs();
        if dist1 < 5.0 { val += 0.6 * (1.0 - dist1 / 5.0); }

        let dist2 = (i as f32 - peak2_pos as f32).abs();
        if dist2 < 8.0 { val += 0.8 * (1.0 - dist2 / 8.0); }

        let dist3 = (i as f32 - peak3_pos as f32).abs();
        if dist3 < 3.0 { val += 0.9 * (1.0 - dist3 / 3.0) * (0.5 + 0.5 * (t * 2.0).cos()); }

        fft[i] = val.clamp(0.0, 1.0);
    }

    fft
}

pub async fn run_spectral_loop(ui_handle: slint::Weak<SpectralIngester>) {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    let mut path_string = String::with_capacity(256 * 16);
    let mut t: f32 = 0.0;

    let mut waterfall = WaterfallBuffer::new(120, 400);
    let num_bins = 120;

    loop {
        interval.tick().await;
        t += 0.05;

        let fft_frame = generate_mock_fft(t, num_bins);

        path_string.clear();
        let x_step = 120.0 / fft_frame.len() as f32;
        for (i, &amp) in fft_frame.iter().enumerate() {
            let x = i as f32 * x_step;
            let y = 100.0 - (amp * 100.0);
            if i == 0 { let _ = write!(path_string, "M {:.1} {:.1} ", x, y); }
            else { let _ = write!(path_string, "L {:.1} {:.1} ", x, y); }
        }

        waterfall.push_new_line(&fft_frame);

        let ui_handle_clone = ui_handle.clone();
        let path_copy = path_string.clone();
        let buffer_clone = waterfall.buffer.clone();

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_handle_clone.upgrade() {
                ui.set_spectrum_path(path_copy.into());
                let img = Image::from_rgba8(buffer_clone);
                ui.set_waterfall_image(img);
            }
        });
    }
}

fn load_dummy_icon() -> tray_icon::Icon {
    let width = 32;
    let height = 32;
    let rgba = vec![0u8; (width * height * 4) as usize];
    tray_icon::Icon::from_rgba(rgba, width, height).unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tray_menu = Menu::new();
    let _ = tray_menu.append(&MenuItem::new("Open Spectral HUD", true, None));
    let _ = tray_menu.append(&MenuItem::new("Exit Antigravity", true, None));

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_icon(load_dummy_icon())
        .build()?;

    let ui = SpectralIngester::new()?;

    #[cfg(target_os = "windows")]
    {
        // Placeholder for apply_acrylic_material
    }

    let ui_handle = ui.as_weak();

    tokio::spawn(async move {
        run_spectral_loop(ui_handle).await;
    });

    ui.run()?;
    Ok(())
}
