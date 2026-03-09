#!/bin/bash
sed -i 's/use crate::ml::mamba;/use crate::ml::mamba;\nuse crate::ml::modular_features::{ModularFeatureEncoder, FeatureFlags, SignalFeaturePayload};/' src/main.rs
