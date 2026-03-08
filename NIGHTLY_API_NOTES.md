# API Compatibility Notes — wgpu git + cpal + qdrant-client + Slint git
# Nightly Rust 2024 — generated from build errors encountered 2026-02-27
#
# This file is the ground truth for "what the actual API is" vs what
# docs.rs shows for stable crate versions. Update as APIs evolve.

## wgpu (git trunk, post-0.20)

### enumerate_adapters() is now ASYNC
- Old (0.20): `instance.enumerate_adapters(backends).into_iter().filter(...)`
- New (git):  `instance.request_adapter(&RequestAdapterOptions { ... }).await`
- The async version returns a single best adapter matching the options.
  For explicit multi-adapter enumeration, use the async `enumerate_adapters()` and
  await the Future before calling `.into_iter()`.
- SIREN uses: `request_adapter` with `PowerPreference::HighPerformance`

### DeviceDescriptor has new required fields
- Old (0.20): `{ label, required_features, required_limits }`
- New (git):  adds `memory_hints`, `trace`, `experimental_features`
- Fix: `..Default::default()` fills all new fields.
- SIREN uses: `..Default::default()` on DeviceDescriptor always.

### PipelineLayoutDescriptor: push_constant_ranges → immediate_size
- Old (0.20): `push_constant_ranges: &[]`
- New (git):  field removed. Push constants now specified via `immediate_size: u32`
  which sets the size in bytes of the immediate data block (0 if unused).
- SIREN uses: `immediate_size: 0` (no push constants needed)

### bind_group_layouts in PipelineLayoutDescriptor: &[BindGroupLayout] → &[Option<BindGroupLayout>]
- Old (0.20): `bind_group_layouts: &[&bgl]`
- New (git):  `bind_group_layouts: &[Some(&bgl)]`
  Each slot is Optional to support sparse layouts.

### ComputePipelineDescriptor: entry_point is now Option<&str>
- Old (0.20): `entry_point: "main"`
- New (git):  `entry_point: Some("main")`
  None means "use the single entry point in the shader module" if unambiguous.

### ComputePipelineDescriptor: cache field added
- New (git):  `cache: None`  (or `Some(&pipeline_cache)` for shader caching)
  Must be present. Set to None unless you have a PipelineCache to reference.

### Instance::new() takes &InstanceDescriptor (was InstanceDescriptor by value in 0.20)
- New (git): `Instance::new(&InstanceDescriptor { ... })`

---

## cpal (0.15, latest on crates.io)

### SampleRate is a newtype(u32) — access via Into<u32> not .0
- `input_config.sample_rate()` returns `SampleRate(u32)`
- Access the inner value: `u32::from(input_config.sample_rate()) as f32`
- The `.0` field accessor produces E0610 "u32 is primitive, no fields" on
  the git version where SampleRate may return a plain u32 in some builds.
- Safe pattern works on both: `u32::from(config.sample_rate())`

### device.name() is deprecated
- Old: `device.name()?`
- New: `device.description().map(|d| d.name).unwrap_or_else(|_| "Unknown".into())`
  `description()` returns `Result<DeviceDescription>` where `DeviceDescription`
  has `.name: String`, `.manufacturer: Option<String>`, `.device_type`.
- The `name()` method still compiles but emits a deprecation warning.

---

## qdrant-client (1.9+)

### qdrant_client::prelude REMOVED
- Old (1.8): `use qdrant_client::prelude::*;`
- New (1.9): Import types directly:
  ```rust
  use qdrant_client::Qdrant;               // client (was QdrantClient)
  use qdrant_client::qdrant::{
      CreateCollectionBuilder, Distance, VectorsConfigBuilder,
      PointStruct, UpsertPointsBuilder, SearchPointsBuilder,
  };
  ```

### Client type renamed
- Old: `QdrantClient::from_url("...").build()?`
- New: `Qdrant::from_url("...").build()?`

### Collection creation uses builders
- Old: `client.create_collection(CreateCollection { ... }).await`
- New: `client.create_collection(CreateCollectionBuilder::new(name).vectors_config(...)).await`

### Collection existence check
- New: `client.collection_exists(name).await?` returns bool

### PointStruct constructor
- Old: `PointStruct { id: Some(PointId { ... }), vectors: Some(vec.into()), payload }`
- New: `PointStruct::new(id: impl Into<PointId>, vector: Vec<f32>, payload: HashMap<String, Value>)`

### SearchPoints uses builder
- Old: `client.search_points(SearchPoints { collection_name, vector, limit, ... }).await`
- New: `client.search_points(SearchPointsBuilder::new(collection, vector, limit).with_payload(true)).await`

### Payload Value type: use qdrant_client::qdrant::Value with Kind variants
- `use qdrant_client::qdrant::{Value, value::Kind};`
- String: `Value { kind: Some(Kind::StringValue(s)) }`
- Float:  `Value { kind: Some(Kind::DoubleValue(v)) }`
- Int:    `Value { kind: Some(Kind::IntegerValue(v)) }`

---

## Slint (git master, targeting 1.15.x)

### Properties on non-Text elements
- `vertical-alignment` and `horizontal-alignment` are TEXT-ONLY properties.
  Setting them on Rectangle, HorizontalBox, or custom components → compile error.

### Layout
- Use `HorizontalLayout` / `VerticalLayout` (not HorizontalBox/VerticalBox in .slint)
  OR use `HorizontalBox` / `VerticalBox` from `std-widgets.slint`.
- Children inside layout containers: use `horizontal-stretch` / `vertical-stretch`,
  not explicit `x`, `y`, `width`, `height` (unless overriding with fixed values).

### Dynamic model binding (VecModel)
- Rust: `ui.set_my_list(std::rc::Rc::new(slint::VecModel::from(vec)).into())`
- Slint struct types used in VecModel MUST be declared in .slint:
  ```slint
  struct MyStruct { field: string, value: float }
  ```
  Then in component: `property <[MyStruct]> my-list;`
- For primitive lists: `property <[float]> my-floats;`

### Callbacks
- Rust: `ui.on_my_callback(|arg| { ... });`
- Slint: `callback my-callback(arg-type);`
- Callback arguments are positional, not named, when invoked from Rust.

### No format!() in Slint
- String concatenation: `"prefix" + Math.round(value) + " Hz"`
- Math functions available: `Math.round()`, `Math.abs()`, `Math.sqrt()`, `Math.log()`
- `clamp(v, min, max)` is a builtin, not a method call.

### Image from Rust
- `slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(data, w, h))`
- Bind to: `property <image> my-image;` in Slint

### Git version may expose APIs not in 1.15.1 release — test before using
- `TabWidget`, `Popup`, `Dialog`, `FocusScope`, `GridLayout` — all present
- `Path` element: limited fill support, test rendering before relying on it

---

## Borrow Checker Notes (Rust Nightly 2024)

### Self-borrow conflict: mutable field access while calling &self method
Problem pattern (E0502):
```rust
let phases = &mut self.coherence_phase[i];
phases.push(v);
let stability = self.phase_stability(phases); // ERROR: self borrowed mut AND immut
```

Fix: extract to owned data before calling self methods, OR convert helper methods
to free functions that take the data as parameters:
```rust
{
    let phases = &mut self.coherence_phase[i];
    phases.push(v);
}
let snapshot: Vec<f32> = self.coherence_phase[i].clone();
let stability = phase_stability(&snapshot); // free fn, no self borrow
```

SIREN pattern: bispectrum.rs converts phase_stability(), mean_phase_circular(),
product_hz(), and compute_embedding() to free functions at module level.
This is the correct architectural pattern — these are pure computations that
have no business being methods on the mutable engine struct.
