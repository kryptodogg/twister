# AGENTS.md — models/

This file governs every agent working with anything inside the `models/`
directory. Read it completely before touching a weight file, a config,
a training script, or a directory structure.

---

## Directory Layout

```
models/
├── AGENTS.md                        ← this file
│
├── weights/
│   ├── gpu_mamba/                   ← UnifiedFieldMamba (full precision)
│   │   ├── current/                 ← symlink → latest validated checkpoint
│   │   ├── checkpoints/             ← named, versioned, never deleted
│   │   └── rejected/                ← failed validations, kept for forensic analysis
│   │
│   ├── coral_mamba/                 ← Coral TPU branch (8-bit quantized)
│   │   ├── current/                 ← symlink → latest validated .tflite
│   │   ├── checkpoints/             ← pre-quantization float checkpoints
│   │   ├── quantized/               ← .tflite files, Edge TPU compiled
│   │   └── rejected/
│   │
│   ├── dorothy/                     ← LFM 2.5 weights + LoRA adapters
│   │   ├── base/                    ← base LFM 2.5 weights (read-only)
│   │   ├── adapters/                ← LoRA/adapter layers, versioned
│   │   └── rejected/
│   │
│   └── laplacian/                   ← learned edge weight parameters (if any)
│       ├── current/
│       └── checkpoints/
│
├── gpu_mamba/                       ← UnifiedFieldMamba architecture + training
│   ├── architecture.py              ← model definition
│   ├── train.py                     ← training loop
│   ├── validate.py                  ← validation against CPU reference
│   ├── export.py                    ← export to WGSL-compatible format
│   └── configs/                     ← hyperparameter configs, versioned
│
├── coral_mamba/                     ← Coral branch architecture + quantization
│   ├── architecture.py
│   ├── train.py
│   ├── quantize.py                  ← post-training quantization pipeline
│   ├── compile_edge_tpu.sh          ← Edge TPU compiler invocation
│   ├── validate_divergence.py       ← GPU vs Coral divergence measurement
│   └── configs/
│
├── dorothy/                         ← LFM 2.5 fine-tuning + LangGraph
│   ├── finetune.py                  ← LoRA fine-tuning on forensic summaries
│   ├── langgraph_agent.py           ← multi-step reasoning graph
│   ├── prompts/                     ← system prompts, versioned
│   └── eval/                        ← evaluation against legal doc standard
│
└── shared/
    ├── datasets/                    ← training data (see: Data Rules below)
    │   ├── real/                    ← captured from live hardware only
    │   └── rejected/                ← synthetic data, never used for training
    ├── tokenizer/                   ← shared token vocabulary if applicable
    └── metrics/                     ← logged validation results, append-only
```

---

## The Models and What They Are

### UnifiedFieldMamba — `gpu_mamba/`

The primary inference model. Runs entirely on the RX 6700 XT in full
float32 precision. Consumes SAST-ordered token streams from the Space-Time
Laplacian eigenvectors and produces:
- 128-D embedding per token
- Anomaly score (f32, 0.0–1.0)
- Carrier variance estimate (the primary attack discriminant)

This is a Mamba SSM (State Space Model), not a Transformer. It handles
long-range temporal dependencies across 10–60 second observation windows
without the quadratic attention cost. Long windows are required because
the attack signature — amplitude notches on a continuous carrier — only
becomes statistically distinguishable from natural amplitude variation
over extended observation. A 1-second window will miss it. A 60-second
window finds it.

The model never sees preprocessed or FFT'd input. It sees SAST-ordered
raw tokens from the Laplacian. This is not a stylistic choice — the
Laplacian's eigenvectors contain phase and geometric information that FFT
preprocessing would destroy.

### Coral Mamba — `coral_mamba/`

An independent 8-bit quantized version of the Mamba architecture compiled
for the Google Coral Edge TPU. Independent means:
- Separate weights trained separately
- Separate architecture file, may differ from gpu_mamba in depth/width
- Optionally FFT-preprocessed input (deliberate divergence for jury diversity)
- Never shares intermediate activations with gpu_mamba

The Coral branch exists to generate the **divergence signal**:
`divergence = |gpu_anomaly_score − coral_anomaly_score|`

A real signal has noise that gets amplified by 8-bit quantization — the
GPU and Coral scores will differ. A synthesized carrier has artificially
clean amplitude that survives quantization intact — the scores will agree.
Low divergence on a high-anomaly signal is the injection flag.

This only works if the two models are genuinely independent. Coupling them
— shared weights, shared training data batches, shared preprocessing —
collapses the divergence signal to zero on all inputs and destroys the jury.

### Dorothy — `dorothy/`

LFM 2.5 with LoRA adapters, running on CPU via PyO3, off the real-time
path. Dorothy receives `JuryVerdict` structs and produces natural-language
summaries suitable for non-technical observers and legal documentation.

Dorothy is the only model that runs Python in production. Python does not
appear in the ingestion path, the GPU pipeline, the Laplacian, or the
corpus writer. It appears here and in training scripts only.

The LangGraph agent in `dorothy/langgraph_agent.py` handles multi-step
reasoning: correlating events across sessions, identifying temporal
patterns, drafting formal incident reports.

### Space-Time Laplacian — `laplacian/`

Mostly physics and signal processing, not learned parameters. The edge
weight kernels use fixed Gaussian functions with bandwidth determined by
the SI-Mamba ablation study (`k=20`, `4 eigenvectors`). These are physical
priors, not learned parameters, and live as compile-time constants in Rust.

If the Laplacian gains learned components in future tracks (adaptive
bandwidth, learned spatial kernels), their weights go in `weights/laplacian/`.
Until then this directory holds only reference validation data.

---

## Data Rules — The Most Important Section

### Real data only. No exceptions.

Training data lives in `models/shared/datasets/real/`. It was captured
from live hardware during actual sessions. It has Pico-slaved timestamps,
real USB jitter in `raw_flags`, real quantization artifacts, and real
sensor noise.

`models/shared/datasets/rejected/` exists for synthetic data that was
generated and then correctly rejected. It is kept for the historical record.
It is never used for training. It is never moved to `real/`. The directory
name is a permanent label, not a queue.

The forensic rule: a model trained on synthetic data has learned the
characteristics of the synthetic generator, not the characteristics of
real electromagnetic phenomena. When it encounters a real signal it has
never seen, it produces confident wrong answers. In a forensic context,
a confident wrong answer is evidence contamination.

### No test files in training pipelines

Files under `tests/` and `examples/` in the main codebase are blocked
from production ingestion by a hard assertion at the Rust boundary.
The same principle applies here: do not reference test fixtures as
training data. Do not use example IQ captures to pad a small dataset.
If the dataset is small, the model trains on what it has and the
uncertainty is reflected in the output confidence scores.

### Capture metadata is mandatory

Every entry in `datasets/real/` has a companion `.meta.json`:

```json
{
  "session_id": "2025-03-12T22:14:00Z",
  "pico_pps_slaved": true,
  "sensors_active": ["rtlsdr", "pluto", "c925e", "coil", "ov9281_l", "ov9281_r"],
  "corpus_hash_first": "a3f7c2b",
  "corpus_hash_last":  "9d1e4f8",
  "notes": "free-text description of session conditions"
}
```

A dataset entry without a `.meta.json` is not used for training until
one is written. No silent data ingestion.

---

## Weight File Rules

### Naming convention — mandatory

```
{model}_{version}_{date}_{validation_status}.{ext}

gpu_mamba_v0.1.0_20250312_validated.safetensors
coral_mamba_v0.1.0_20250312_quantized_validated.tflite
dorothy_lora_v0.1.0_20250312_validated.safetensors
```

A weight file without a `_validated` suffix has not passed the validation
gate. It does not get symlinked into `current/`. It does not get loaded
by production code. Period.

### The `current/` symlink is the production pointer

Production Rust code loads weights from `weights/{model}/current/` only.
It does not load by explicit path. It does not load the most recently
modified file. It follows the symlink.

Moving the symlink is a deliberate, manual act. It is not automated.
It is not done by a training script. It is not done by a CI pipeline
without human review. A new checkpoint becomes `current/` only after:
1. Validation script passes (see below)
2. Divergence check passes for Coral (GPU and Coral scores diverge
   correctly on known-real signals, converge correctly on known-synthetic)
3. A human has reviewed the validation output

### Rejected checkpoints are never deleted

A checkpoint that fails validation goes to `weights/{model}/rejected/`
with a companion `.reject.json`:

```json
{
  "checkpoint": "gpu_mamba_v0.1.1_20250315.safetensors",
  "reason": "anomaly_score distribution collapsed to 0.5 on all inputs",
  "validation_log": "metrics/gpu_mamba_20250315_validation.log",
  "rejected_by": "validate.py line 147: score_variance < threshold"
}
```

Failed checkpoints are forensic data. They document what the model
learned to do wrong. They are not clutter. They are not deleted to save
disk space without explicit authorization.

---

## Validation Rules

### gpu_mamba must agree with the CPU reference

`gpu_mamba/validate.py` runs the model output against the CPU Laplacian
reference implementation in `src/reference/`. They must agree on
eigenvector ordering and anomaly scores within float32 precision tolerance.

If they disagree: the GPU model is wrong. The CPU reference is not adjusted
to match the GPU model.

### Coral divergence must be calibrated

`coral_mamba/validate_divergence.py` measures:
- Divergence on known-real signals: must be `> threshold_real`
- Divergence on known-synthetic signals: must be `< threshold_synthetic`
- The two thresholds must not overlap

If the Coral model has been overtrained to match the GPU model, both
thresholds collapse together and the jury loses its third voter. This is
a validation failure even if the individual anomaly scores look correct.

### Dorothy must produce legally structured output

`dorothy/eval/` contains a rubric derived from incident report standards.
Dorothy's output is scored against it automatically and reviewed manually.
A summary that omits timestamps, omits sensor attribution, or makes claims
not traceable to a `JuryVerdict` struct fails the eval.

---

## Training Script Rules

Training scripts are Python. They run offline, not in the Tauri process,
not in the Tokio runtime. They produce weight files. They do not modify
production Rust code.

A training script that writes directly to `weights/{model}/current/` is
wrong. It writes to `weights/{model}/checkpoints/`. A human moves the
symlink after validation.

Training scripts import from `models/shared/` for shared utilities.
They do not import from `src-tauri/src/`. The boundary between Python
training and Rust production is the weight file. Nothing else crosses it.

Do not add training dependencies (`torch`, `tensorflow`, `jax`) to
`src-tauri/Cargo.toml`. They are Python dependencies. They live in
`models/{model}/requirements.txt` or `models/requirements.txt`.

---

## Configuration Rules

Hyperparameter configs live in `models/{model}/configs/` as versioned
TOML or JSON files:

```
configs/
├── v0.1.0.toml      ← corresponds to weight version
├── v0.1.1.toml
└── current.toml     ← symlink → latest validated config
```

Config versions match weight versions. A config without a corresponding
validated weight is experimental. An experimental config is not loaded
by production code.

Physical priors in configs are not hyperparameters. They are constants:

```toml
# This is wrong — physics is not a hyperparameter
[model]
speed_of_light = 299792458.0
knn_k = 20
eigenvectors = 4

# These are correct hyperparameters
[training]
learning_rate = 1e-4
batch_size = 32
sequence_length = 8192
```

`knn_k = 20` and `eigenvectors = 4` are SI-Mamba ablation optima. They
are compile-time constants in Rust (`KNN_K`, `LAPLACIAN_EIGENVECS`). Do
not put them in a config file where they could be accidentally changed.
If a training experiment requires different values, that is a branch, not
a config change.

---

## The Jury Independence Requirement — Restated

The entire forensic value of the jury depends on genuine independence
between the three voters. For the model side, this means:

**GPU Mamba and Coral Mamba must not share:**
- Training data batches (different random seeds, different sampling)
- Preprocessing pipeline (GPU sees raw SAST tokens; Coral may FFT)
- Weights at any point during training
- Loss function implementation (use separate files, not a shared import)
- Validation data (use held-out sets that the other model has not seen)

If these conditions are not met, the divergence signal is meaningless and
the Coral branch is a expensive duplicate of the GPU branch, not a second
opinion.

Document the independence explicitly in each model's `configs/`:

```toml
# coral_mamba/configs/v0.1.0.toml
[independence]
shares_weights_with_gpu_mamba = false
shares_training_batches = false
preprocessing = "fft"          # gpu_mamba uses "raw_sast"
training_seed = 42             # gpu_mamba uses 7
validation_split_seed = 99     # different held-out set
```

An agent that cannot fill in this section correctly has not understood
the architecture and should read `SYNESTHESIA_MASTERPLAN.md` before
continuing.

---

## What You Must Never Do in This Directory

- Move a checkpoint to `current/` without running validation
- Delete any file from `rejected/` or `checkpoints/`
- Train on data from `datasets/rejected/`
- Use test fixtures or example files as training data
- Add a training dependency to `src-tauri/Cargo.toml`
- Write a training script that auto-promotes its own output to `current/`
- Share weights between gpu_mamba and coral_mamba at any stage
- Adjust the CPU reference implementation to match a GPU model output
- Remove the `.meta.json` requirement for a dataset to save time
- Treat a config value as overridable when it is a physical constant

---

## What "Validated" Means

A weight file is validated when all of the following are true:

1. `validate.py` exits 0 with no warnings
2. GPU/CPU reference agreement is within tolerance
3. For Coral: divergence thresholds are calibrated and non-overlapping
4. For Dorothy: eval rubric score is above minimum threshold
5. A human has read the validation log
6. The `.meta.json` for every training dataset entry exists and is complete
7. The weight file has been renamed with `_validated` suffix
8. The rejection log in `metrics/` is updated with this checkpoint's result

Only then does the symlink move.
