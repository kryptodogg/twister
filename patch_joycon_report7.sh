sed -i 's/raw.gyro.x/raw.gyro_1/g' examples/joycon_wand.rs
sed -i 's/raw.gyro.y/raw.gyro_2/g' examples/joycon_wand.rs
sed -i 's/raw.gyro.z/raw.gyro_3/g' examples/joycon_wand.rs
sed -i 's/raw.accel.x/raw.accel_x/g' examples/joycon_wand.rs
sed -i 's/raw.accel.y/raw.accel_y/g' examples/joycon_wand.rs
sed -i 's/raw.accel.z/raw.accel_z/g' examples/joycon_wand.rs
sed -i 's/process_imu_data(imu_data,/process_imu_data(imu_data.data[0],/' examples/joycon_wand.rs
