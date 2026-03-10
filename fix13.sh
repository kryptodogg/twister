sed -i 's/let handle = app.as_weak();/ /g' examples/chronos_slate.rs
sed -i 's/use raw_window_handle::{HasWindowHandle, RawWindowHandle};/use raw_window_handle::RawWindowHandle;/g' examples/chronos_slate.rs
