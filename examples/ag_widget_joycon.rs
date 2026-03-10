slint::include_modules!();
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = JoyconWidget::new()?;
    let backend = ui.global::<JoyconBackend>();

    // Mocking the JoyconHandler from Track B Addendum
    let ui_handle = ui.as_weak();

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(16)); // 60 Hz polling
        let mut phase = 0.0_f32;

        loop {
            interval.tick().await;
            phase += 0.1;

            if let Some(ui) = ui_handle.upgrade() {
                let backend = ui.global::<JoyconBackend>();
                backend.set_connected(true);
                // Simulate Gyro twist and tilt
                backend.set_gyro_roll(phase.sin() * 90.0);
                backend.set_gyro_pitch((phase * 0.5).cos() * 90.0);
                backend.set_accel_x(phase.sin() * 2.0);
            }
        }
    });

    ui.run()?;
    Ok(())
}
