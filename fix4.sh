sed -i 's/wgpu::Maintain::Wait/wgpu::Maintain::wait()/g' examples/chronos_slate.rs
sed -i 's/crate::ChronosBus/ChronosBus/g' examples/chronos_slate.rs
sed -i 's/import { ChronosBus, ChronosSlate }/export { ChronosBus, ChronosSlate }/g' examples/chronos_slate.rs
