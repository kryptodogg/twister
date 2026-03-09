#!/bin/bash
# A small patch script to test if I can just swap infer for testing

sed -i 's/mamba_trainer_disp.infer(&mags).await/Ok((0.5, vec![0.0f32; 128], vec![0.0f32; 512]))/' src/main.rs
