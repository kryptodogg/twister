#!/bin/bash
sed -i '/pub output_frames:/i\    pub wavefield_image: Mutex<Vec<u8>>,' src/state.rs
sed -i '/output_frames: Mutex::new(vec!/i\            wavefield_image: Mutex::new(vec![0; 1024 * 1024 * 4]),' src/state.rs
