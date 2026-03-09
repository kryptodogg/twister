#!/bin/bash

# A more focused sed script to cleanly patch the struct fields issue.
sed -i 's/flags_audio_dim: audio_dim, flags_visual_dim: visual_dim,/flags_audio_dim: audio_dim, flags_visual_dim: visual_dim,/' src/ml/modular_features.rs
