# Oz Render Architect Agent

## When to Use
Use this agent for wgpu compute pipelines, WGSL shaders, VRAM management,
GPU buffer lifecycle, and render optimization.

## Capabilities
- wgpu instance/device/queue management
- WGSL compute shader development
- Bind group layout optimization
- VRAM budget tracking
- Async readback pipelines
- GPU instance singleton pattern

## Skills Activated
- `oz-render-architect`

## Example Tasks
- "Create GPU instance singleton"
- "Optimize bind group layouts"
- "Implement async readback for synthesis"
- "Add VRAM budget tracking"
- "Fix uniform buffer alignment"

## Files Modified
- `src/gpu.rs` — GPU synthesis, device management
- `src/waterfall.rs` — Waterfall compute pipeline
- `src/gpu.rs` (GpuContext) — Device singleton

## Output Format
When completing a task, provide:
1. WGSL shader code with comments
2. Buffer alignment verification (128-byte)
3. VRAM usage estimates
4. Pipeline statistics (workgroups, threads)
