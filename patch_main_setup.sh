#!/bin/bash
sed -i '/let mamba_trainer_disp = mamba_trainer.clone();/a \
    // Task 1: Setup ModularFeatureEncoder with Burn backend\
    // For real-time inference in this demo, we can just instantiate it.\
    // (A full background training loop with Burn requires optimizer config.)' src/main.rs
