# Skill: validate_mcp_server

## Overview

Validates Model Context Protocol (MCP) server implementation for Glinda orchestrator. Ensures JSON-RPC 2.0 compliance, tool registration, and telemetry streaming.

## Applicable Agents

- `glinda-mcp-orchestrator`
- `synesthesia-holographic-ui`
- `oz-render-architect`

## Execution

```bash
# Run MCP server validation
python scripts/validate_mcp.py --server <SERVER_ENDPOINT> --tools <TOOL_LIST>

# Example: Validate local MCP server
python scripts/validate_mcp.py --server http://localhost:8080/mcp --tools get_simulation_status,get_haptic_readback,set_environment_wetness
```

## Validation Criteria

### Pass Conditions
- JSON-RPC 2.0 compliance: `jsonrpc`, `id`, `method`, `params` fields present
- Tool registration: All tools registered with valid schemas
- Read tools response time: < 10 ms
- Write tools validation: Bounds checking before execution
- Telemetry streaming: subscribe/listChanged pattern working
- Error handling: Proper JSON-RPC error codes (-32600 to -32603)

### Fail Conditions
- Missing JSON-RPC 2.0 fields
- Unregistered tools
- Response time > 10 ms for read tools
- Write tools without bounds validation
- Telemetry subscription failures
- Improper error codes

## Detection Patterns

The validator detects MCP implementations by:
- Type names: `McpServer`, `JsonRpcRequest`, `ToolSchema`
- Function names: `register_tool`, `handle_request`, `subscribe`
- Variable patterns: `jsonrpc`, `tool_catalog`, `subscription_id`

## Output Format

```json
{
  "server": "http://localhost:8080/mcp",
  "tests": [
    {
      "name": "json_rpc_compliance",
      "request": {"jsonrpc": "2.0", "id": 1, "method": "get_simulation_status"},
      "response": {"jsonrpc": "2.0", "id": 1, "result": {...}},
      "fields_present": ["jsonrpc", "id", "result"],
      "status": "PASS"
    },
    {
      "name": "tool_registration",
      "tool": "get_simulation_status",
      "schema_valid": true,
      "description": "Return current simulation state",
      "params_schema": {},
      "response_schema": "SimulationStatus",
      "status": "PASS"
    },
    {
      "name": "read_tool_latency",
      "tool": "get_simulation_status",
      "request_count": 100,
      "avg_latency_ms": 3.2,
      "p99_latency_ms": 8.5,
      "target_ms": 10.0,
      "status": "PASS"
    },
    {
      "name": "write_tool_validation",
      "tool": "set_environment_wetness",
      "valid_input": {"wetness": 0.5},
      "invalid_input": {"wetness": 1.5},
      "rejected_invalid": true,
      "error_code": -32602,
      "status": "PASS"
    },
    {
      "name": "telemetry_subscription",
      "subscribe_request": {"resource": "particle_count"},
      "list_changed_response": {"delta": [{"op": "replace", "path": "/particle_count", "value": 1000000}]},
      "subscription_active": true,
      "update_rate_hz": 60.0,
      "status": "PASS"
    }
  ],
  "summary": {
    "total": 5,
    "passed": 5,
    "failed": 0,
    "avg_latency_ms": 3.2,
    "tools_registered": 8,
    "subscriptions_active": 1
  }
}
```

## JSON-RPC 2.0 Message Format

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "get_simulation_status",
  "params": {}
}

// Response (success)
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "particle_count": 1000000,
    "fps": 60.0,
    "frame_time_ms": 16.5
  }
}

// Response (error)
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid params: wetness must be between 0.0 and 1.0"
  }
}
```

## MCP Tool Schema

```rust
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: JsonSchema,
    pub output_schema: JsonSchema,
}

// Example: set_environment_wetness
ToolSchema {
    name: "set_environment_wetness".to_string(),
    description: "Update RF-BSDF wetness parameter".to_string(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "wetness": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0
            }
        },
        "required": ["wetness"]
    }),
    output_schema: json!({
        "type": "object",
        "properties": {
            "ack": {"type": "boolean"}
        }
    }),
}
```

## Standard Error Codes

| Code | Message | Description |
|------|---------|-------------|
| -32600 | Invalid Request | JSON-RPC 2.0 format error |
| -32601 | Method Not Found | Tool not registered |
| -32602 | Invalid Params | Parameter validation failed |
| -32603 | Internal Error | Server execution error |

## Telemetry Streaming Pattern

```rust
// Subscribe to resource updates
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "subscribe",
  "params": {"resource": "particle_count"}
}

// Server responds with subscription ID
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {"subscription_id": "sub_001"}
}

// Request state delta
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "listChanged",
  "params": {"subscription_id": "sub_001"}
}

// Server responds with JSON Patch
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "deltas": [
      {"op": "replace", "path": "/particle_count", "value": 1000000}
    ]
  }
}
```

## Timeout

Maximum execution time: 30 seconds

## Integration

This skill is called automatically by validation hooks after editing:
- `domains/cognitive/glinda/src/mcp/**/*.rs`
- `domains/cognitive/glinda/src/server.rs`
- Any file containing `McpServer` or `JsonRpcRequest`

## Related Files

- `scripts/validate_mcp.py` - Main MCP validator
- `domains/cognitive/glinda/src/mcp/` - MCP server implementation
- `conductor/tracks/oz_mcp_server_20260221/plan.md` - Implementation plan

## References

- JSON-RPC 2.0 Specification: https://www.jsonrpc.org/specification
- Model Context Protocol: https://modelcontextprotocol.io
- "Building MCP Servers in Rust", Project Oz Internal
