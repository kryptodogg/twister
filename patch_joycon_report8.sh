sed -i 's/raw.gyro_1/raw.data\[0\].gyro_1/g' examples/joycon_wand.rs
sed -i 's/raw.gyro_2/raw.data\[0\].gyro_2/g' examples/joycon_wand.rs
sed -i 's/raw.gyro_3/raw.data\[0\].gyro_3/g' examples/joycon_wand.rs
sed -i 's/raw.accel_x/raw.data\[0\].accel_x/g' examples/joycon_wand.rs
sed -i 's/raw.accel_y/raw.data\[0\].accel_y/g' examples/joycon_wand.rs
sed -i 's/raw.accel_z/raw.data\[0\].accel_z/g' examples/joycon_wand.rs
sed -i 's/process_imu_data(imu_data.data\[0\],/process_imu_data(imu_data,/' examples/joycon_wand.rs
