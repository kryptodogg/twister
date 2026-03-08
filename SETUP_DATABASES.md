# SIREN v0.2 — Forensic Database Stack
# Windows 11, AMD RX 6700 XT, no WSL2 GPU passthrough
#
# ── Qdrant: native Windows binary (GPU-accelerated via Vulkan) ────────────────
# WSL2 does not support AMD GPU passthrough, so skip Docker for Qdrant entirely.
# Use the native Windows Qdrant binary which talks to the GPU via Vulkan directly.
#
# Setup:
#   1. Download the latest qdrant Windows release (GPU build):
#      https://github.com/qdrant/qdrant/releases
#      Get:  qdrant-x86_64-pc-windows-msvc.zip  (or the -gpu variant when available)
#
#   2. Create config.yaml next to qdrant.exe:
#
#      storage:
#        storage_path: ./storage
#      service:
#        host: 0.0.0.0
#        http_port: 6333
#        grpc_port: 6334
#      gpu:
#        indexing:
#          enabled: true          # uses AMD RX 6700 XT via Vulkan
#
#   3. Run: qdrant.exe --config-path config.yaml
#      Qdrant will print the detected Vulkan device — should show "AMD Radeon RX 6700 XT"
#
#   4. REST API: http://localhost:6333
#      gRPC:     localhost:6334
#
# ── Neo4j: native Windows service ────────────────────────────────────────────
# Neo4j runs fine under Docker without GPU, but native avoids WSL2 overhead.
#
# Setup:
#   1. Download Neo4j Community 5.x:
#      https://neo4j.com/deployment-center/
#      Get: neo4j-community-5.x.x-windows.zip
#
#   2. Set password (run once from the neo4j bin/ directory):
#      neo4j-admin dbms set-initial-password siren_forensic_2024
#
#   3. Optional — enable APOC (for graph algorithms):
#      Copy apoc-5.x.x-core.jar into plugins/
#      Add to neo4j.conf:
#        dbms.security.procedures.unrestricted=apoc.*
#
#   4. Start: neo4j console
#      OR install as a Windows service: neo4j install-service && neo4j start
#
#   5. Bolt: bolt://localhost:7687
#      Browser UI: http://localhost:7474
#
# ── Memory tuning for 64 GB RAM ──────────────────────────────────────────────
# neo4j.conf:
#   server.memory.heap.initial_size=1g
#   server.memory.heap.max_size=8g
#   server.memory.pagecache.size=4g
#
# ── Verification ─────────────────────────────────────────────────────────────
# Once both are running, SIREN will connect automatically at startup.
# If either is unavailable, SIREN continues without it (forensic logging
# still works via JSONL files in ./forensic_log/).
#
# Quick health checks (PowerShell):
#   Invoke-WebRequest http://localhost:6333/healthz   # Qdrant
#   Invoke-WebRequest http://localhost:7474           # Neo4j browser
