# Hermes P2P Testing

6-node P2P mesh for testing Gossipsub message propagation.

---

## TL;DR

```bash
cd p2p-testing
just quickstart    # Build, start mesh, test PubSub
```

That's it. The mesh starts, tests run automatically, and you'll see if PubSub propagation works.

---

## Common Operations

```bash
# Start the mesh
just start                       # Starts 6 nodes, waits for mesh formation

# Test PubSub propagation
just test-pubsub-propagation     # Sends message from node 1 → all others

# Monitor
just logs                        # View all node logs
just status                      # Show node endpoints

# Stop
just stop                        # Stop nodes (preserves data)
just clean                       # Stop and delete everything
```

### Fast Restart (When Binary Unchanged)

```bash
docker compose up -d             # Start without rebuilding
docker compose restart           # Restart without rebuilding
```

> **Note:** `just start` always rebuilds Docker images. Use `docker compose` directly to skip rebuilds.

---

## Troubleshooting

**Test failed?**
```bash
sleep 30 && just test-pubsub-propagation    # Wait for mesh, retry
just troubleshoot                            # Full diagnostics report
```

**Need detailed help?** See [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md)

---

## How It Works

**Why 6 nodes?**
Gossipsub uses `mesh_n=6` by default. With fewer nodes, you get "Mesh low" warnings and incomplete propagation. With 6 nodes, each connects to 5 others forming a complete mesh.

**Network Topology:**
- 6 nodes in full mesh (15 bidirectional connections)
- IPs: 172.20.0.10 through 172.20.0.15
- HTTP ports: 5000, 5002, 5004, 5006, 5008, 5010
- Persistent peer IDs stored in Docker volumes

**What happens in a test:**
1. Node 1 receives HTTP POST → publishes to PubSub topic "doc-sync"
2. Gossipsub propagates message through mesh
3. All other nodes (2-6) receive and validate the message

---

## Prerequisites

- Docker & Docker Compose
- [Just](https://just.systems)
- Rust toolchain (for building Hermes)
- [Earthly](https://earthly.dev) (Mac/Windows only - auto-detected)

**Platform support:** Builds automatically detect your OS and use the appropriate method. Mac/Windows users get containerized builds via Earthly (slower but works everywhere).

Check prerequisites: `just validate-prereqs`

---

## All Commands

Run `just` to see all available commands, or:

**Setup & Start:**
- `just quickstart` - Complete setup and test (first-time users)
- `just start` - Start nodes (prompts for rebuild)
- `just start-ci` - Start nodes (CI mode: always rebuilds)

**Testing:**
- `just test-pubsub-propagation` - Test message propagation (interactive)
- `just test-ci` - Full CI test suite

**Monitoring:**
- `just logs` - All node logs
- `just status` - Node endpoints
- `just check-connectivity` - P2P connectivity report

**Management:**
- `just stop` - Stop nodes (preserves data)
- `just restart` - Restart nodes
- `just clean` - Delete everything (⚠️ deletes peer IDs)

**Troubleshooting:**
- `just troubleshoot` - Generate diagnostics report
- `just init-bootstrap` - Reset bootstrap config (auto-runs after clean)

---

## CI/CD Pipeline

```bash
just start-ci && just test-ci && just clean
```

- Always starts from clean state
- Runs full validation suite
- Cleans up after completion

> **TODO:** Integrate this CI workflow into GitHub Actions runners for automated testing on PRs

---

## Files

- `justfile` - All commands and documentation
- `docker-compose.yml` - 6-node configuration
- `Dockerfile` - Container image
- `TROUBLESHOOTING.md` - Detailed debugging guide

---

## Architecture Details

<details>
<summary>Click to expand network topology</summary>

```
Full Mesh: Each node connects to all 5 others
Total connections: 15 bidirectional links

Node 1 (172.20.0.10) ←→ Node 2 (172.20.0.11)
Node 1 (172.20.0.10) ←→ Node 3 (172.20.0.12)
Node 1 (172.20.0.10) ←→ Node 4 (172.20.0.13)
Node 1 (172.20.0.10) ←→ Node 5 (172.20.0.14)
Node 1 (172.20.0.10) ←→ Node 6 (172.20.0.15)
... (and so on for all node pairs)
```

**Features:**
- Persistent IPFS keypairs (stable peer IDs)
- Bootstrap retry logic (automatic reconnection)
- Gossipsub v1.2 PubSub protocol
- Full mesh connectivity (172.20.0.0/16 network)

</details>

---

**For detailed documentation and troubleshooting, see [`justfile`](justfile) and [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md)**
