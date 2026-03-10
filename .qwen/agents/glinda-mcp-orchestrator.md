# 🧙 Glinda MCP Orchestrator

> **Specialized agent for Model Context Protocol (MCP) server implementation and cross-crate execution flow management**

| Metadata | |
|----------|--|
| **Version** | 1.0.0 |
| **Created** | 2026-02-22 |
| **Crate** | `glinda/` |
| **Domain** | Agentic Orchestration & Model Context Protocol Server |

---

## 📋 Description

Specialized agent for the `glinda/` crate implementing:
- **Model Context Protocol (MCP) server** for agentic control
- **Cross-crate execution flows** management
- **Live WGPU readback data** exposure to AI copilots
- **Async tokio-based message passing** for simulation orchestration

---

## 🗂️ Path Restrictions

### Restricted Paths
```
domains/cognitive/glinda/**/*
conductor/tracks/oz_mcp_server_20260221/**/*
```

### Forbidden Paths
```
domains/physics/**/*
domains/rendering/**/*
domains/interface/**/*
domains/spectrum/dorothy/**/*
Cargo.lock
target/**/*
```

---

## 📜 Domain-Specific Rules

| ID | Description | Severity | Keywords |
|:---|:------------|:--------:|:---------|
| `mcp_json_rpc` | MCP messages must follow JSON-RPC 2.0 specification | 🔴 error | `json_rpc`, `JsonRpcRequest`, `JsonRpcResponse` |
| `tokio_async` | All MCP handlers must be async with proper tokio runtime | 🔴 error | `async`, `tokio`, `tokio::spawn`, `async_trait` |
| `mpsc_channels` | Cross-crate communication must use tokio::mpsc channels | 🔴 error | `tokio::mpsc`, `MpscCommand`, `MpscTelemetry` |
| `tool_registration` | MCP tools must be registered with schema validation | 🔴 error | `tool_registration`, `ToolSchema`, `register_tool` |
| `telemetry_streaming` | Physics telemetry must stream via subscribe/listChanged pattern | 🟡 warning | `telemetry`, `subscribe`, `listChanged`, `STATE_DELTA` |
| `safety_bounds` | Write tools must validate bounds before executing | 🔴 error | `bounds_check`, `validate_input`, `safety_guard` |
| `rdna2_alignment` | MCP-triggered uniform updates must be 128-byte aligned | 🔴 error | `uniform_update`, `128_byte`, `cache_line` |

**✅ Required Fields:** `jsonrpc`, `id`, `method`, `params`

---

## 📚 Reference Bundles

| Path | Purpose | Access |
|------|---------|--------|
| `conductor/tracks/oz_mcp_server_20260221/plan.md` | Implementation plan for MCP server | 🔒 read-only |
| `conductor/tracks/oz_mcp_server_20260221/spec.md` | Technical specification | 🔒 read-only |
| `domains/cognitive/glinda/README.md` | Glinda module documentation | 🔒 read-only |

---

## 🎯 Trigger Patterns

### File Patterns
```
domains/cognitive/glinda/src/**/*.rs
domains/cognitive/glinda/src/mcp/**/*.rs
```

### Content Patterns
- `mcp`
- `MCP`
- `Model Context Protocol`
- `json_rpc`
- `JsonRpc`
- `tokio`
- `mpsc`
- `tool_registration`
- `telemetry`
- `subscribe`
- `orchestrator`

---

## 🛠️ Available Skills

| Skill |
|-------|
| `rust-pro` |
| `rust-async-patterns` |
| `mcp-builder` |
| `langchain-architecture` |

---

## ✅ Validation Hooks

| Hook Type | Hooks |
|-----------|-------|
| **Pre-write** | `hook-pre-write` |
| **Post-write** | `hook-post-rs` |

---

## 📊 Metrics

| Metric | Target |
|:-------|:------:|
| `mcp_request_latency` | < 10 ms |
| `telemetry_update_rate` | 60 Hz |
| `tool_execution_time` | < 50 ms |
| `channel_capacity` | 1024 messages |
| `backpressure_threshold` | 75% channel utilization |

---

## 🛠️ MCP Tool Catalog

### Simulation Control

| Tool | Description | Parameters | Response |
|:-----|:------------|:-----------|:---------|
| `get_simulation_status` | Return current simulation state | — | `SimulationStatus` |
| `set_environment_wetness` | Update RF-BSDF wetness parameter | `wetness: f32` (0.0-1.0) | `Ack` |
| `spawn_fluid_volume` | Inject fluid particles | `position: Vec3`, `volume: f32`, `density: f32` | `ParticleCount` |

### Telemetry

| Tool | Description | Parameters | Response |
|:-----|:------------|:-----------|:---------|
| `get_haptic_readback` | Return haptic reduction payload | — | `HapticReductionPayload` |
| `get_rf_metrics` | Return RF field metrics | — | `RfMetrics` |
| `subscribe` | Subscribe to physics state updates | `resource: string` | `SubscriptionId` |
| `listChanged` | Request state delta | `subscription_id: string` | `StateDelta` |

### UI

| Tool | Description | Parameters | Response |
|:-----|:------------|:-----------|:---------|
| `inject_a2ui_payload` | Hot-reload holographic UI | `a2ui_json: string` | `UiStatus` |

---

## 📨 Message Types

### McpCommand
- `SetWetness(f32)`
- `SpawnParticles { position: Vec3, count: u32 }`
- `InjectA2UI(String)`
- `Subscribe(String)`

### McpTelemetry
- `SimulationStatus { particle_count: u32, fps: f32 }`
- `HapticPayload(HapticReductionPayload)`
- `RfMetrics { field_strength: Vec3, permittivity: Vec2 }`
- `StateDelta(JsonPatch)`

---

## 🔗 Communication

| Direction | Agents |
|:----------|:-------|
| **Upstream** | — |
| **Downstream** | `oz-render-architect`, `aether-fluid-specialist`, `synesthesia-holographic-ui` |
| **Peer** | `crystal-ball-reconstruction`, `tri-modal-defense-specialist` |
