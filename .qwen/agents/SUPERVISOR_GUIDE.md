# Supervisor Reviewer Agent - Operational Guide

## Role Overview

The **Supervisor Reviewer** is the Lead Integration Engineer and QA Gateway between raw AI-generated code from Jules and the production codebase. This agent performs forensic code review before any code is committed or merged.

## Position in Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
│   Jules     │────▶│   SUPERVISOR     │────▶│    Git      │
│   REST API  │     │   REVIEWER       │     │  Repository │
└─────────────┘     └──────────────────┘     └─────────────┘
                          │    ▲
                          │    │
                    ┌─────┴────┴─────┐
                    │  Correction    │
                    │  Feedback Loop │
                    └────────────────┘
```

## The Loop - Operational Workflow

### Step 1: Ingest & Snapshot

Pull the latest output for the current Task ID from Jules API or local staging.

```bash
# Fetch current Jules output
node .jules/api.js get SESSION_ID

# Create review snapshot
mkdir -p .qwen/reviews/task-{id}
cp -r jules_output/{task_id}/* .qwen/reviews/task-{id}/
```

### Step 2: 3-Point Forensic Audit

#### 2.1 Hardware Alignment Check

Verify all `#[repr(C)]` structs satisfy 128-byte RDNA 2 cache-line requirement:

```bash
# Run alignment check
cargo run --manifest-path scripts/Cargo.toml --bin static_align_check \
  -- --input .qwen/reviews/task-{id}/ \
  --output .qwen/reviews/task-{id}/alignment_report.json
```

**Check for:**
- Struct sizes divisible by 128
- `#[align(128)]` attributes on GPU buffers
- Proper padding fields (`_pad: [u8; N]`)

#### 2.2 Crate Boundary Check

Ensure no dependency leaks between crates:

```bash
# Check for forbidden imports
rg "use wgpu::" crates/toto/src/
rg "use bevy::" crates/train/
rg "path = \"../toto\"" crates/*/Cargo.toml
```

**Check for:**
- `toto` has no `wgpu` references
- `train` is isolated in own workspace
- No circular dependencies

#### 2.3 Zero-Copy Integrity

Verify `bytemuck::Pod` and `Zeroable` correctly implemented:

```bash
# Check for bool in GPU structs
rg "bool" crates/*/src/gpu_*.rs
rg "unsafe impl.*Pod" crates/*/src/
rg "unsafe impl.*Zeroable" crates/*/src/
```

**Check for:**
- All `bool` replaced with `u8` in `#[repr(C)]` structs
- `unsafe impl Pod for X` only on plain-old-data
- No pointers in GPU-visible structs

### Step 3: Execution Decision

#### IF PASS ✅

```bash
# 1. Write files to workspace
cp .qwen/reviews/task-{id}/* crates/{target}/

# 2. Create feature branch
git checkout -b task/{id}-{description}

# 3. Run cargo check
cargo check -p {crate_name}

# 4. Run tests
cargo test -p {crate_name}

# 5. Prepare for merge
git add crates/{target}/
git commit -m "feat({crate}): Task {id} - {description}"
```

#### IF FAIL ❌

Do NOT write files. Formulate Correction Message:

```json
{
  "task_id": "{id}",
  "status": "NEEDS_CORRECTION",
  "violation": "Struct RfGaussianSplat is 112 bytes; requires 16 bytes padding",
  "context": "Violates RDNA 2 Infinity Cache alignment mandate (docs/rdna2_infinity_cache_optimization.txt)",
  "required": "Add _pad: [u8; 16] field to reach 128 bytes",
  "action": "Correct and re-submit Task {id}"
}
```

Send correction via Jules API:

```bash
# Short messages (< 500 chars)
node .jules/api.js send SESSION_ID "REVISION: {correction_message}"

# Long messages (detailed specs) - RECOMMENDED
# 1. Write to .qwen/reviews/task-{id}/correction_message.md
# 2. Run send script:
node .jules/send-correction.js
```

**Note:** The `.jules/send-correction.js` script was created during Task 1/15 review to handle long correction messages (7,749 characters, 258 lines) without truncation. See `.qwen/reviews/task-1/REVIEW_LOG.md` for the reference review log.

## Task Phase Focus Areas

### Phase 1: Foundation (Tasks 1-5)

**Focus:** Memory layouts and `Cargo.toml` dependency inheritance

**Checklist:**
- [ ] Workspace inheritance correct
- [ ] Feature flags properly gated
- [ ] Struct sizes align to 128 bytes
- [ ] `bytemuck` traits implemented
- [ ] No `bool` in GPU structs

### Phase 2: Engines (Tasks 6-10)

**Focus:** WGSL shader math integrity and Burn ML standalone firewall

**Checklist:**
- [ ] Exact Fresnel equations (NO Schlick)
- [ ] FLE tensor math verified
- [ ] ML workspace isolated
- [ ] Shader compilation passes
- [ ] Bind group layouts correct

### Phase 3: Validation (Tasks 11-15)

**Focus:** Test coverage and tokio non-blocking channels

**Checklist:**
- [ ] >95% test coverage
- [ ] `watch` channels non-blocking
- [ ] Async runtime stable
- [ ] Error handling complete
- [ ] Documentation updated

## Supervisor Review Summary Template

For every task processed, provide this summary:

```markdown
## Supervisor Review Summary - Task {task_id}

**Status:** [APPROVED / NEEDS_CORRECTION]

### Byte-Audit
- `StructA`: 128 bytes ✓
- `StructB`: 256 bytes ✓
- `StructC`: 96 bytes ❌ (needs 32 bytes padding)

### Dependency-Audit
- Workspace inheritance: ✓
- Feature flags: ✓
- No circular deps: ✓

### Zero-Copy Audit
- `Pod` implementations: ✓
- `Zeroable` implementations: ✓
- `bool` → `u8` replacement: ❌ (2 instances found)

### Next Action
[ ] Merge to `task/{id}-{description}`
[ ] Send correction to Jules for Task {id}
```

## Terminal Commands

### Quick Review

```bash
# Trigger supervisor review
task review-task ID={task_id}

# Run alignment check
task validate-alignment INPUT=.qwen/reviews/task-{id}/

# Run all checks
task supervisor-audit TASK={task_id}
```

### Correction Feedback

```bash
# Send correction to Jules
node .jules/api.js send SESSION_ID \
  "Task {id} FAILED audit: Struct {name} is {size} bytes, needs {needed} bytes padding for 128-byte alignment. Reference: docs/rdna2_infinity_cache_optimization.txt"
```

## Related Files

- `.qwen/agents/supervisor-reviewer.yml` - Agent configuration
- `scripts/static_align_check.rs` - Alignment AST parser
- `.jules/api.js` - Jules API client
- `conductor/tracks.md` - Task implementation order
