sed -i 's/let manager = JoyConManager::new().unwrap();/let manager = JoyConManager::get_instance();/' examples/joycon_wand.rs
sed -i 's/let devices = manager.new_devices();/let devices = { let lock = manager.lock(); match lock { Ok(m) => m.new_devices(), Err(_) => return, } };/' examples/joycon_wand.rs
