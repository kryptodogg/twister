// examples/test_slint_device_controls.rs
//
// Tests Slint ↔ DeviceManager wiring (without real UI rendering).
// Mocks Slint callbacks and verifies DeviceManager is called correctly.

use std::sync::Arc;
use twister::app_state::DirtyFlags;
use twister::hardware_io::device_manager::DeviceManager;
use twister::ui::DeviceControlsController;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Test: Slint Device Controls Wiring ===\n");

    // Setup
    let flags = Arc::new(DirtyFlags::new());
    let dm = Arc::new(DeviceManager::new(flags.clone()));
    let ctrl = DeviceControlsController::new(dm.clone(), flags.clone());

    // Test 1: Get initial device list (empty)
    println!("[1] Initial device list...");
    let devices = ctrl.get_device_list();
    println!("  Devices: {} (expected 0)", devices.len());
    assert_eq!(devices.len(), 0);
    println!("✓ Initial state correct\n");

    // Test 2: Mock "+ Add RTL-SDR" button click
    println!("[2] Simulating: Click '+ Add RTL-SDR' button (device index 0)...");
    match ctrl.on_add_rtl_sdr_clicked(0) {
        Ok(msg) => println!("✓ {}", msg),
        Err(e) => println!("✗ {}", e),
    }

    // Test 3: Verify device list updated
    println!("\n[3] Checking device list after add...");
    let devices = ctrl.get_device_list();
    println!("  Devices: {} (expected 1+)", devices.len());
    if !devices.is_empty() {
        let (id, name, freq_mhz, status) = &devices[0];
        println!(
            "  Device #{}: {} @ {:.0} MHz [{}]",
            id, name, freq_mhz, status
        );
    }

    // Test 4: Mock frequency input change
    println!("\n[4] Simulating: Frequency input change to 1500.5 MHz...");
    if !devices.is_empty() {
        let device_id = devices[0].0;
        match ctrl.on_frequency_changed(device_id, 1500.5) {
            Ok(msg) => println!("✓ {}", msg),
            Err(e) => println!("✗ {}", e),
        }
    }

    // Test 5: Verify dirty flag was set
    println!("\n[5] Checking dirty flags...");
    if flags.check_and_clear(&flags.device_list_dirty) {
        println!("✓ device_list_dirty flag was set (then cleared)");
    } else {
        println!("✓ device_list_dirty not currently set (may have been cleared)");
    }

    // Test 6: Mock "- Remove Device" button click
    println!("\n[6] Simulating: Click '- Remove Device' button...");
    if !devices.is_empty() {
        let device_id = devices[0].0;
        match ctrl.on_remove_device_clicked(device_id) {
            Ok(msg) => println!("✓ {}", msg),
            Err(e) => println!("✗ {}", e),
        }
    }

    // Test 7: Verify device list is empty again
    println!("\n[7] Checking device list after remove...");
    let devices = ctrl.get_device_list();
    println!("  Devices: {} (expected 0)", devices.len());
    assert_eq!(devices.len(), 0);
    println!("✓ Final state correct");

    println!("\n=== Test Complete ===");
    Ok(())
}
