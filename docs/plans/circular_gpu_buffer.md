# Track: Circular GPU Buffer & Phase State Persistence

## 1. Specification
Refactor the GPU synthesis and waterfall engines to replace the O(N*M) memory shift with a Virtual Ring Buffer and persistent state tracking.

## 2. Implementation Plan
- [ ] Task: Phase 1.1 - Create GpuState Struct
- [ ] Task: Phase 1.2 - Allocate STORAGE Buffer
- [ ] Task: Phase 1.3 - Update Synthesis Shader
- [ ] Task: Conductor - User Manual Verification 'Phase 1'
