#!/bin/bash

# Restore the original line
sed -i 's/Ok((0.5, vec!\[0.0f32; 128\], vec!\[0.0f32; 512\]))/mamba_trainer_disp.infer(\&mags).await/' src/main.rs

# Let's insert the new modular_features logic right above the mamba inference block
# I'll create a background task that does the Burn inference independently.
