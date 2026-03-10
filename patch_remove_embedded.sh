sed -i '/^\/\/ Embedded struct for testing/,/^}$/d' src/ml/pattern_discovery.rs
sed -i 's/use rustfft::{FftPlanner, num_complex::Complex};/use rustfft::{FftPlanner, num_complex::Complex};\nuse super::data_contracts::ForensicEventData;/g' src/ml/pattern_discovery.rs
