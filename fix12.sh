sed -i 's/let _ = self.device.poll/let _ = self.device.poll/g' examples/chronos_slate.rs
sed -i 's/self.device.poll/let _ = self.device.poll/g' examples/chronos_slate.rs
sed -i 's/let _ = let _ = /let _ = /g' examples/chronos_slate.rs
