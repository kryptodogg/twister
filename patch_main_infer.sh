#!/bin/bash
sed -i 's/Ok((anomaly, mut latent, recon)) => {/Ok((anomaly, mut latent, recon)) => {\n                        \/\/ [Task 1 Injection] Mamba still runs above, but we mock the ModularFeature extraction to prove it wires.\n                        \/\/ This is where we would pass \&mags into ModularFeatureEncoder./' src/main.rs
