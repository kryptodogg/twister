// examples/test_radio_device_open.rs
//
// Tests FFI wrapper: opening device, querying properties, closing cleanly.
// Requires RTL-SDR plugged in or Pluto+ on USB.
// If no device, returns error (expected).

use twister::safe_sdr_wrapper::{RadioDevice, RadioDeviceType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Test: Radio Device FFI Wrapper ===\n");

    // Test 1: Try to open RTL-SDR device 0
    println!("[1] Attempting to open RTL-SDR device (index=0)...");
    match RadioDevice::open_rtl_sdr(0) {
        Ok(mut dev) => {
            println!("✓ Device opened successfully");
            println!("  Device type: {:?}", dev.device_type());
            println!("  Center freq: {} Hz", dev.center_freq());
            println!("  Sample rate: {} Hz", dev.sample_rate());

            // Test 2: Tune frequency
            println!("\n[2] Tuning to 1.5 GHz...");
            let freq_1_5ghz = 1_500_000_000u64;
            match dev.tune_freq(freq_1_5ghz) {
                Ok(_) => {
                    println!("✓ Tuned to {} Hz", dev.center_freq());
                }
                Err(e) => {
                    println!("✗ Tune failed: {}", e);
                }
            }

            // Test 3: Clean drop
            println!("\n[3] Closing device...");
            drop(dev);
            println!("✓ Device closed (Drop impl called)");
        }
        Err(e) => {
            println!("✗ Failed to open RTL-SDR: {}", e);
            println!("  (Expected if device not plugged in)");
        }
    }

    // Test 4: Try Pluto+ (if compiled with feature)
    #[cfg(feature = "pluto-plus")]
    {
        println!("\n[4] Attempting to open Pluto+ device...");
        match RadioDevice::open_pluto_plus(0) {
            Ok(dev) => {
                println!("✓ Pluto+ opened");
                println!("  Device type: {:?}", dev.device_type());
            }
            Err(e) => {
                println!("✗ Failed to open Pluto+: {}", e);
                println!("  (Expected if device not present)");
            }
        }
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
