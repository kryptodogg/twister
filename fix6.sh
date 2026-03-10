sed -i 's/wgpu::PollType::Wait/wgpu::PollType::Wait/g' examples/chronos_slate.rs
sed -i 's/wgpu::PollType::Wait/wgpu::PollType::wait()/g' examples/chronos_slate.rs
