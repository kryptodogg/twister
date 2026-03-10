sed -i 's/joycon_rs::prelude::IMUData/joycon_rs::joycon::driver::input_report_mode::StandardFullMode::IMUData/' examples/joycon_wand.rs
sed -i 's/joycon_rs::prelude::SensorCalibration/joycon_rs::joycon::device::calibration::imu::IMUCalibration/' examples/joycon_wand.rs
