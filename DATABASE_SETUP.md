# Twister v0.4 - Database Setup Guide (Native Windows)

## Quick Start

1. **Install Neo4j:**
   ```powershell
   winget install Neo4j.Neo4j
   ```
   Or download from: https://neo4j.com/download-center/#community

2. **Install Qdrant:**
   ```powershell
   winget install qdrant
   ```
   Or download from: https://github.com/qdrant/qdrant/releases (download `qdrant-x86_64-pc-windows-msvc.zip`)

3. **Run the startup script:**
   ```
   START_DATABASES.bat
   ```

---

## Manual Installation

### Neo4j 5.23 Community

1. Download: https://neo4j.com/download-center/#community
2. Extract to `C:\neo4j`
3. Edit `C:\neo4j\conf\neo4j.conf`:
   ```
   dbms.security.auth_enabled=true
   dbms.default_listen_address=0.0.0.0
   ```
4. Start: `C:\neo4j\bin\neo4j.bat console`

**Access:**
- Browser: http://localhost:7474
- Bolt: localhost:7687
- Default credentials: neo4j / neo4j (change on first login)

---

### Qdrant

1. Download latest release: https://github.com/qdrant/qdrant/releases
2. Extract to `C:\qdrant`
3. Create `C:\qdrant\config.yaml`:
   ```yaml
   storage:
     storage_path: ../databases/qdrant/storage
     snapshots_path: ../databases/qdrant/snapshots
   
   service:
     grpc_port: 6334
     http_port: 6333
   ```
4. Start: `C:\qdrant\qdrant.exe`

**Access:**
- REST API: http://localhost:6333
- gRPC: localhost:6334
- Dashboard: http://localhost:6333/dashboard

---

## Verify Installation

```powershell
# Test Neo4j
curl http://localhost:7474

# Test Qdrant
curl http://localhost:6333/healthz
```

---

## Stop Databases

```powershell
# Neo4j
C:\neo4j\bin\neo4j.bat stop

# Qdrant
# Press Ctrl+C in the Qdrant console window
```

---

## Data Persistence

All data is stored in:
- `databases\neo4j\` - Neo4j graph database
- `databases\qdrant\` - Qdrant vector database

These directories are preserved across restarts.
