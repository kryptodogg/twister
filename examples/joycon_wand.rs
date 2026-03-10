slint::include_modules!();
use joycon_rs::prelude::*;
use tokio::time::Duration;
use nalgebra as na;
use joycon_rs::joycon::device::calibration::imu::IMUCalibration;
use std::fmt::Write;

// Constants for Joy-Con sensor conversion
const GYRO_SENSITIVITY: f32 = 0.070;  // mps/digit @ ±2000dps
const ACCEL_SENSITIVITY: f32 = 0.000244; // g/digit @ ±8g

/// Maps raw IMU bytes to physical SI units (G-force and Degrees/sec)
pub fn process_imu_data(raw: joycon_rs::prelude::input_report_mode::standard_full_mode::IMUData, calib: &IMUCalibration) -> (na::Vector3<f32>, na::Vector3<f32>) {
    // 1. Apply Factory Calibration Offsets
    let (accel_offset_x, accel_offset_y, accel_offset_z, gyro_offset_x, gyro_offset_y, gyro_offset_z) = match calib {
        IMUCalibration::Available { acc_origin_position, gyro_origin_position, .. } => {
            (
                acc_origin_position.x as f32,
                acc_origin_position.y as f32,
                acc_origin_position.z as f32,
                gyro_origin_position.x as f32,
                gyro_origin_position.y as f32,
                gyro_origin_position.z as f32,
            )
        },
        _ => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    };

    let gyro_x = (raw.data[0].gyro_1 as f32 - gyro_offset_x) * GYRO_SENSITIVITY;
    let gyro_y = (raw.data[0].gyro_2 as f32 - gyro_offset_y) * GYRO_SENSITIVITY;
    let gyro_z = (raw.data[0].gyro_3 as f32 - gyro_offset_z) * GYRO_SENSITIVITY;

    let accel_x = (raw.data[0].accel_x as f32 - accel_offset_x) * ACCEL_SENSITIVITY;
    let accel_y = (raw.data[0].accel_y as f32 - accel_offset_y) * ACCEL_SENSITIVITY;
    let accel_z = (raw.data[0].accel_z as f32 - accel_offset_z) * ACCEL_SENSITIVITY;

    (
        na::Vector3::new(accel_x, accel_y, accel_z),
        na::Vector3::new(gyro_x, gyro_y, gyro_z)
    )
}

pub struct WandOrientation {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl WandOrientation {
    /// Updates orientation based on gyro velocity and Delta-T
    pub fn update(&mut self, gyro_vel: na::Vector3<f32>, dt: f32) {
        // Integrate angular velocity to get angular displacement
        self.roll += gyro_vel.x * dt;
        self.pitch += gyro_vel.y * dt;
        self.yaw += gyro_vel.z * dt;

        // Wrap angles to [0, 360] for the UI
        self.roll %= 360.0;
        self.pitch %= 360.0;
        self.yaw %= 360.0;
    }
}

pub async fn start_joycon_loop(ui_handle: slint::Weak<JoyconWandApplet>) {
    let manager = JoyConManager::get_instance();
    let devices = { let lock = manager.lock(); match lock { Ok(m) => m.new_devices(), Err(_) => return, } };

    // 1. Pick the first available Joy-Con
    if let Some(device) = devices.into_iter().next() {
        let driver = match SimpleJoyConDriver::new(&device) {
            Ok(d) => d,
            Err(_) => return,
        };

        // 2. Enable IMU and set high-performance report mode
        let mut standard_full_mode = match StandardFullMode::new(driver) {
            Ok(m) => m,
            Err(_) => return,
        };

        let calib = match device.lock() {
            Ok(d) => d.imu_user_calibration().clone(),
            Err(_) => return,
        };

        let mut orientation = WandOrientation { roll: 0.0, pitch: 0.0, yaw: 0.0 };
        let dt = 0.0166; // 60 Hz

        let ui_handle_1 = ui_handle.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_handle_1.upgrade() {
                let status = ui.global::<JoyconStatus>();
                status.set_connected(true);
            }
        });

        let mut path_buffer = String::with_capacity(128 * 16);
        let mut shake_history = std::collections::VecDeque::with_capacity(50);

        loop {
            // 3. Read the HID report
            if let Ok(report) = standard_full_mode.read_input_report() {
                // Get trigger state
                // Right Trigger maps to ZR on Right JoyCon or equivalent
                let trigger_r_val = if report.common.pushed_buttons.contains(Buttons::ZR) { 255.0 } else { 0.0 };

                let imu_data = report.extra;
                let (accel, gyro) = process_imu_data(imu_data, &calib);

                // Update the orientation state
                orientation.update(gyro, dt);

                let shake_magnitude = (accel.x.powi(2) + accel.y.powi(2) + accel.z.powi(2)).sqrt();

                // Maintain a rolling history for the sparkline (max 50 points)
                if shake_history.len() >= 50 {
                    shake_history.pop_front();
                }
                shake_history.push_back(shake_magnitude);

                path_buffer.clear();
                if !shake_history.is_empty() {
                    write!(&mut path_buffer, "M 0 {}", 40.0 - (shake_history[0] * 10.0).clamp(0.0, 40.0)).unwrap();
                    for (i, &mag) in shake_history.iter().enumerate().skip(1) {
                        let x = (i as f32 / 49.0) * 140.0; // Scale X to width (140px)
                        let y = 40.0 - (mag * 10.0).clamp(0.0, 40.0); // Scale Y to height (40px), invert for SVG
                        write!(&mut path_buffer, " L {} {}", x, y).unwrap();
                    }
                }

                // 4. Reactive UI Update
                let ui_handle_2 = ui_handle.clone();
                let r = orientation.roll;
                let p = orientation.pitch;
                let y = orientation.yaw;
                let ax = accel.x;
                let ay = accel.y;
                let az = accel.z;
                let path_str = path_buffer.clone();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle_2.upgrade() {
                        let status = ui.global::<JoyconStatus>();
                        status.set_gyro_roll(r);
                        status.set_gyro_pitch(p);
                        status.set_gyro_yaw(y);
                        status.set_accel_x(ax);
                        status.set_accel_y(ay);
                        status.set_accel_z(az);
                        status.set_trigger_r(trigger_r_val);
                        status.set_heterodyne_strength(shake_magnitude);
                        status.set_sparkline_path(slint::SharedString::from(path_str));
                    }
                });
            }
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = JoyconWandApplet::new()?;
    let ui_handle = ui.as_weak();

    // Spawn the Joy-Con loop
    tokio::spawn(async move {
        start_joycon_loop(ui_handle).await;
    });

    ui.run()?;
    Ok(())
}
