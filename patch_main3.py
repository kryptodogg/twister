import re

with open("src/main.rs", "r") as f:
    content = f.read()

# Replace crate::ml with twister::ml
content = content.replace("crate::ml", "twister::ml")

# Fix Mutex in _start_trainer_loop
content = content.replace("state: Arc<Mutex<crate::state::AppState>>", "state: std::sync::Arc<std::sync::Mutex<twister::state::AppState>>")

# Fix `state_disp.get_feature_flags()` at line 246 where state_disp is not defined yet
bad_tdoa = """            let feature_flags = state_disp.get_feature_flags();
            engine.ingest(&tdoa_rx);"""
good_tdoa = """            engine.ingest(&tdoa_rx);"""
content = content.replace(bad_tdoa, good_tdoa)

# Fix crate::state::AppState to twister::state::AppState
content = content.replace("crate::state::AppState", "twister::state::AppState")

# Fix crate::ml::modular_features to twister::ml::modular_features
content = content.replace("crate::ml::modular_features", "twister::ml::modular_features")


with open("src/main.rs", "w") as f:
    f.write(content)
