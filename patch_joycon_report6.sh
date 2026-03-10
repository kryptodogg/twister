cat << 'INNER_EOF' > examples/joycon_wand.rs
slint::include_modules!();
use joycon_rs::prelude::*;
use tokio::time::Duration;
use nalgebra as na;
use joycon_rs::joycon::device::calibration::imu::IMUCalibration;

// Constants for Joy-Con sensor conversion
const GYRO_SENSITIVITY: f32 = 0.070;  // mps/digit @ ±2000dps
const ACCEL_SENSITIVITY: f32 = 0.000244; // g/digit @ ±8g

/// Maps raw IMU bytes to physical SI units (G-force and Degrees/sec)
pub fn process_imu_data(raw: joycon_rs::joycon::input_report_mode::IMUData, calib: &IMUCalibration) -> (na::Vector3<f32>, na::Vector3<f32>) {
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

    let gyro_x = (raw.gyro.x as f32 - gyro_offset_x) * GYRO_SENSITIVITY;
    let gyro_y = (raw.gyro.y as f32 - gyro_offset_y) * GYRO_SENSITIVITY;
    let gyro_z = (raw.gyro.z as f32 - gyro_offset_z) * GYRO_SENSITIVITY;

    let accel_x = (raw.accel.x as f32 - accel_offset_x) * ACCEL_SENSITIVITY;
    let accel_y = (raw.accel.y as f32 - accel_offset_y) * ACCEL_SENSITIVITY;
    let accel_z = (raw.accel.z as f32 - accel_offset_z) * ACCEL_SENSITIVITY;

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

        if let Some(ui) = ui_handle.upgrade() {
            let _ = slint::invoke_from_event_loop(move || {
                let status = ui.global::<JoyconStatus>();
                status.set_connected(true);
            });
        }

        loop {
            // 3. Read the HID report
            if let Ok(report) = standard_full_mode.read_input_report() {
                // Get trigger state
                // Right Trigger maps to ZR on Right JoyCon or equivalent
                let trigger_r_val = if report.common.pushed_buttons.contains(Buttons::ZR) { 255.0 } else { 0.0 };

                let imu_data = report.extra.data[0];
                let (accel, gyro) = process_imu_data(imu_data, &calib);

                // Update the orientation state
                orientation.update(gyro, dt);

                let shake_magnitude = (accel.x.abs() + accel.y.abs() + accel.z.abs()) / 24.0; // simple mock

                // 4. Reactive UI Update
                if let Some(ui) = ui_handle.upgrade() {
                    let _ = slint::invoke_from_event_loop(move || {
                        let status = ui.global::<JoyconStatus>();
                        status.set_gyro_roll(orientation.roll);
                        status.set_gyro_pitch(orientation.pitch);
                        status.set_gyro_yaw(orientation.yaw);
                        status.set_accel_x(accel.x);
                        status.set_accel_y(accel.y);
                        status.set_accel_z(accel.z);
                        status.set_trigger_r(trigger_r_val);
                        status.set_heterodyne_strength(shake_magnitude);
                    });
                }
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
INNER_EOF
