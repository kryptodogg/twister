#!/bin/bash
sed -i 's/let s = particle_streamer.clone();/let s = particle_streamer.clone();/' src/main.rs
# wait, ParticleStreamLoader was wrapped in Arc in main.rs before my previous change? Let's check how it's instantiated.
