# AGENTS.md — tests/

This file governs every agent writing, modifying, or running tests in
this directory. Read it before writing a single `#[test]` attribute.

---

## Tests Are Proofs

A test is not a sanity check. A test is not a demonstration. A test is
not a regression guard bolted on after the fact. A test is a formal claim
about the behavior of the system — a proposition that the code either
satisfies or falsifies.

A proof has exactly the structure it needs. It does not repeat itself.
It does not prove the same proposition ten ways. It does not introduce
assumptions that weaken what it claims to prove. And critically: **a new
proof does not invalidate existing proofs**. If adding a test causes
another test to fail, one of three things is true:

1. The new test is wrong
2. The existing test was testing the wrong proposition
3. The code changed in a way that broke a real invariant

In all three cases, the answer is to understand which proposition is
false — not to delete the failing test, not to `#[ignore]` it, not to
mark it `#[should_panic]` as a workaround. Find the false proposition.
Fix it.

---

## Directory Layout

```
tests/
├── AGENTS.md                    ← this file
│
├── output/                      ← all test output files land here
│   ├── .gitkeep                 ← directory tracked, contents gitignored
│   └── *.{log,json,bin,...}     ← generated at runtime
│
├── errors/                      ← captured error output from failing runs
│   ├── .gitkeep
│   └── *.{log,txt,...}          ← gitignored, kept for local debugging
│
├── fixtures/                    ← static input data for tests
│   ├── iq/                      ← real captured IQ samples (small, curated)
│   ├── frames/                  ← real OV9281 frame data (small, curated)
│   ├── verdicts/                ← known-good JuryVerdict structs
│   └── README.md                ← provenance for every fixture file
│
├── integration/                 ← cross-module propositions
│   ├── ingestion_to_gpu.rs      ← RawIQPoint reaches VRAM correctly
│   ├── jury_pipeline.rs         ← all three voters produce verdicts
│   ├── corpus_integrity.rs      ← append-only, hash chain valid
│   └── color_operator.rs        ← freq_to_hue invertibility across all bands
│
├── unit/                        ← single-module, single-proposition tests
│   ├── field_particle.rs        ← 128-byte law, named reservations
│   ├── atomic_f32.rs            ← NaN round-trip, ordering semantics
│   ├── timestamp.rs             ← Pico PPS discipline, no SystemTime
│   ├── sam_gate.rs              ← single CPU→GPU cross verified
│   └── physical_constants.rs   ← speed of light, wavefront width, etc.
│
├── reference/                   ← CPU reference vs GPU WGSL agreement
│   ├── laplacian.rs             ← eigenvectors agree within float tolerance
│   ├── spmv.rs                  ← sparse matrix-vector multiply
│   └── gram_schmidt.rs          ← orthogonality of eigenvector set
│
└── forensic/                    ← evidence integrity propositions
    ├── no_synthetic_data.rs     ← assert no test fixtures reach production
    ├── corpus_append_only.rs    ← no overwrites, no deletes, fsync verified
    └── hash_chain.rs            ← SHA-256 chain unbroken across sessions
```

---

## Output Discipline

**No test writes to the repository root. No test writes to `src/`.
No test writes to `models/`. All test output goes to `tests/output/`
or `tests/errors/`. No exceptions.**

```rust
// WRONG — pollutes the root
let path = PathBuf::from("test_output.bin");

// WRONG — pollutes src
let path = PathBuf::from("src/debug_frame.raw");

// RIGHT — stays in tests/output
let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/output")
    .join(format!("{}_output.bin", test_name));
```

Both `tests/output/` and `tests/errors/` are gitignored at the content
level. The directories are tracked via `.gitkeep`. A clean `git status`
after running the full test suite shows no new files outside these two
directories.

`tests/errors/` accumulates across runs intentionally — a failing CI run
leaves artifacts that can be inspected without re-running the suite.

---

## The Non-Redundancy Rule

Before writing a new test, search the existing suite:

```bash
grep -r "FieldParticle" tests/
grep -r "freq_to_hue" tests/
grep -r "sam_gate" tests/
```

If the proposition you are about to test is already covered, do not add
a second test for it. One of three outcomes applies:

1. **The existing test covers it exactly.** Do not add yours.
2. **The existing test covers it partially.** Extend the existing test
   to cover the gap. Do not create a parallel test.
3. **Your test covers a genuinely different proposition.** Add it, and
   document in its doc comment why it is distinct from existing coverage.

Ten tests for `freq_to_hue` means nine of them are noise. The one that
matters — the proposition that `hue_to_freq(freq_to_hue(f)) == f` within
float tolerance across the full sensor band — is the proof. Write that one.

The correct question before adding a test is not "does this test pass?"
It is: **what proposition does this test prove, and is that proposition
already proven elsewhere?**

---

## A New Test Must Not Break Existing Tests

If adding or modifying a test causes a previously passing test to fail,
stop immediately. Do not proceed. Do not delete the failing test. Do not
mark it ignored. Diagnose:

```
New test T2 causes existing test T1 to fail.

Ask:
  Does T2 mutate shared state?     → T2 is wrong. Tests are isolated.
  Does T2 change a fixture file?   → T2 is wrong. Fixtures are immutable.
  Does T2 reveal T1 was wrong?     → Fix T1's proposition, document why.
  Does T2 reveal a code bug?       → Fix the code. Both tests must pass.
  Does T2 depend on T1's output?   → T2 is wrong. Tests do not chain.
```

Tests are not allowed to depend on each other's execution order. Rust's
test runner does not guarantee order. A test that passes in isolation
and fails in a suite violates isolation. Fix the isolation, not the order.

---

## Test Isolation Rules

### No shared mutable state between tests

```rust
// WRONG — static mut leaks across all tests in the binary
static mut COUNTER: u32 = 0;

// WRONG — LazyLock with interior mutability leaks between tests
static DEVICE: LazyLock<Mutex<GpuDevice>> = LazyLock::new(|| ...);

// RIGHT — each test constructs what it needs, locally
#[test]
fn test_sam_write() {
    let (device, queue) = create_test_device(); // fresh, local, dropped at end
}
```

### No test touches the production corpus

The production forensic corpus is never read or written by a test. Tests
that verify corpus behavior use a temporary directory:

```rust
#[test]
fn corpus_is_append_only() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = Corpus::new(dir.path()); // test corpus, not production
    // ...
} // dir drops here, cleaned up automatically
```

### No test loads from `weights/current/`

Tests that exercise model inference use weight fixtures from
`tests/fixtures/` — small, known-good, committed to the repository.
They never follow the `current/` symlink. A test that breaks when
production weights are updated is testing weights, not code. Weight
validation belongs in `models/`, not in `tests/`.

### Fixtures are immutable

Files in `tests/fixtures/` are never written by a test. They are inputs,
not outputs. If a test needs a modified version of a fixture, it copies
to `tests/output/`, modifies the copy, and the copy is cleaned up with
the rest of the output directory.

Every fixture has a provenance entry in `tests/fixtures/README.md`:
- Where it came from (real hardware capture, generated reference data)
- What session or corpus hash it corresponds to
- Why it is the right fixture for the tests that use it

A fixture without a provenance entry is not used in any test until
the README is updated.

---

## Physical Constants in Tests

Tests never redefine physical constants. They import from `src/`.

```rust
// WRONG — local approximation, silently wrong
const SPEED_OF_LIGHT: f64 = 3e8;

// RIGHT — the exact compile-time constant
use crate::constants::SPEED_OF_LIGHT_M_S;
```

A test that passes with an approximate constant and fails with the exact
one has been proving a weaker proposition than it claimed. The exact
constant is always used. There are no "close enough" values in a forensic
system.

---

## GPU Tests

Tests that exercise WGSL shaders require a real GPU adapter. They are
gated behind a feature flag and skipped in environments without GPU access:

```rust
#[test]
#[cfg(feature = "gpu_tests")]
fn laplacian_eigenvectors_agree() {
    let adapter = pollster::block_on(get_test_adapter());
    // ...
}
```

```bash
# Run GPU tests locally
cargo test --features gpu_tests
```

Every GPU test has a CPU reference counterpart in `tests/reference/` that
runs in CI without GPU access. The CPU reference test is the proposition.
The GPU test verifies that the WGSL implementation satisfies the same
proposition.

If the CPU reference fails: the proposition or the reference is wrong.
Fix it before looking at the GPU test.
If the CPU reference passes and the GPU test fails: the WGSL is wrong.
The CPU reference is never adjusted to match the GPU output.

---

## What a Good Test Looks Like

A good test has four parts, in this order:

```rust
/// Proves that `freq_to_hue` is invertible across the full sensor band.
///
/// Proposition: for all f in [F_MIN_HZ, F_MAX_HZ],
///   hue_to_freq(freq_to_hue(f)) ≈ f within relative float32 tolerance.
///
/// This is the Rosetta Stone property: any frequency maps to a hue and
/// recovers exactly. If this fails, cross-sensor correlation is broken
/// at the color operator level — every downstream analysis is invalid.
#[test]
fn freq_to_hue_is_invertible() {
    use crate::constants::{F_MIN_HZ, F_MAX_HZ};
    use crate::color::{freq_to_hue, hue_to_freq};

    // 1. ARRANGE — the input space, covering sensor band boundaries
    let test_frequencies: &[f64] = &[
        F_MIN_HZ,       // infrasound floor — 1 Hz
        60.0,           // powerline fundamental
        1_000.0,        // audio midrange
        100e6,          // RTL-SDR midband
        2.4e9,          // WiFi / Pluto+ lower
        5.8e9,          // Pluto+ upper band
        F_MAX_HZ,       // visible light ceiling — 700 THz
    ];

    for &f in test_frequencies {
        // 2. ACT
        let hue      = freq_to_hue(f);
        let recovered = hue_to_freq(hue);

        // 3. ASSERT — relative tolerance, not absolute
        let epsilon = f * (f32::EPSILON as f64) * 4.0;
        assert!(
            (recovered - f).abs() < epsilon,
            "freq_to_hue not invertible at {f:.3e} Hz: \
             got {recovered:.3e}, delta {:.3e}",
            (recovered - f).abs()
        );
    }
    // 4. CLEANUP — pure function, nothing to clean
}
```

The doc comment states the proposition before the code does. If you
cannot write the proposition in plain language, you do not yet understand
what you are testing.

---

## What a Bad Test Looks Like

```rust
// BAD: no proposition, tests a default value, not a system property
#[test]
fn test_field_particle() {
    let p = FieldParticle::default();
    assert!(p.energy >= 0.0);
}
```

`energy >= 0.0` on a default-constructed struct tests the default
initializer, not a system invariant. The real proposition — that energy
is non-negative after ingestion from any live sensor — requires a real
ingestion path, real input, and an assertion on the output of that path.

Default-value tests add noise to the suite without adding confidence.

---

## The Block List

These propositions cannot be meaningfully tested here and should not
be attempted:

- **"The model produces accurate results"** — accuracy is a training
  metric. Validate in `models/`, not here.
- **"Performance is above X fps"** — benchmarks live in `benches/`.
  A timing assertion in a test is a flaky test waiting to happen.
- **"The UI renders correctly"** — visual regression belongs in a
  dedicated visual testing framework.
- **Tests that `unwrap()` silently** — a panic in a test is allowed,
  but only with `expect("reason this cannot fail in this context")`.
  Silent `unwrap()` makes failures undiagnosable.
- **Tests that exist to hit coverage metrics** — coverage is a
  byproduct of good tests, not the goal. A test written to reach 80%
  coverage proves nothing.

---

## Running the Suite

```bash
# Full suite (no GPU)
cargo test

# With GPU tests (requires AMD Vulkan driver, ReBAR active)
cargo test --features gpu_tests

# Specific module
cargo test unit::field_particle

# Capture errors for inspection
cargo test 2> tests/errors/$(date +%Y%m%d_%H%M%S).log

# Verify the root is clean afterward
git status  # should show no new files outside tests/output/ and tests/errors/
```

---

## The One Question

Before committing any test, ask:

**What proposition does this test prove, and does that proposition need
to be true for the system to be correct?**

If the answer is "it checks that the code runs without crashing," that
is a smoke test. Smoke tests belong in `examples/`, not in `tests/`.
If the answer is a clear, falsifiable claim about system behavior, the
test belongs here.

Write fewer tests. Make each one count.
