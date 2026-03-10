sed -i 's/joycon_rs::joycon::driver::input_report_mode::StandardFullMode::IMUData/joycon_rs::joycon::input_report_mode::IMUData/' examples/joycon_wand.rs
sed -i 's/calib.offset()/calib.offset/' examples/joycon_wand.rs
sed -i 's/raw.gyro.x()/raw.gyro.x/' examples/joycon_wand.rs
sed -i 's/raw.gyro.y()/raw.gyro.y/' examples/joycon_wand.rs
sed -i 's/raw.gyro.z()/raw.gyro.z/' examples/joycon_wand.rs
sed -i 's/raw.accel.x()/raw.accel.x/' examples/joycon_wand.rs
sed -i 's/raw.accel.y()/raw.accel.y/' examples/joycon_wand.rs
sed -i 's/raw.accel.z()/raw.accel.z/' examples/joycon_wand.rs
