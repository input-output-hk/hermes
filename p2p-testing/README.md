# Hermes P2P Testing

6-node P2P mesh for testing Gossipsub message propagation.

---

## TL;DR

```bash
cd p2p-testing
just quickstart    # Build, start mesh, test PubSub
```

That's it.
The mesh starts, tests run automatically, and you'll see if PubSub propagation works.

> **Note:** `quickstart` uses existing binaries if present.
> If you changed code, run `just build-all` first to rebuild.

---

## Common Operations

```bash
# Start the mesh
just start                       # Starts 6 nodes, waits for mesh formation

# Test PubSub propagation
just test-pubsub-propagation     # Sends message from node 1 → all others
# Test bidirectional sync
just test-bidirectional-sync     # Basic test for nodes bidirectional behavior
# Test node late joiner
just test-late-join-sync         # Test node late join where it sync with keepalive timer

# Monitor
just logs                        # View all node logs
just status                      # Show node endpoints

# Stop
just stop                        # Stop nodes (preserves data)
just clean                       # Stop and delete everything
```

---

## Writing Custom Tests

The `test-pubsub-propagation` command is just one test we wrote to ensure PubSub works in a Hermes mesh.
**You can add your own tests** to validate whatever aspects of the P2P infrastructure matter to your use case.

> **Note:** These are basic bash/curl tests for initial validation.
> Once HTTP API endpoints are finalized, we'll add a proper API testing framework
> (e.g., Postman collections, REST-assured, or similar) for comprehensive integration testing.

Extend the justfile with custom test commands to validate different P2P behaviors:

### Test Examples

**Peer Connectivity:**

```justfile
# Verify all nodes are responding
test-peer-connectivity:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Testing peer connectivity..."
    for NODE in {1..6}; do
      PORT=$((7878 + (NODE-1)*2))
      if curl -s -f http://localhost:$PORT/health > /dev/null 2>&1; then
        echo "✅ Node $NODE (port $PORT) responding"
      else
        echo "❌ Node $NODE not responding"
        exit 1
      fi
    done
    echo "✅ All nodes healthy"
```

**Mesh Resilience:**

```justfile
# Test mesh recovery when nodes restart
test-mesh-resilience:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Testing mesh resilience..."

    echo "  → Stopping node 3..."
    docker compose stop hermes-node3
    sleep 5

    echo "  → Testing propagation with 5 nodes..."
    just test-pubsub-propagation || echo "  (Expected: degraded)"

    echo "  → Restarting node 3..."
    docker compose start hermes-node3
    sleep 10

    echo "  → Verifying full mesh recovered..."
    just test-pubsub-propagation
    echo "✅ Mesh resilience validated"
```

**Message Throughput:**

```justfile
# Measure message throughput
test-throughput COUNT="100":
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Testing throughput with {{COUNT}} messages..."

    START=$(date +%s)
    for i in $(seq 1 {{COUNT}}); do
      curl -s -X POST http://localhost:7878/api/doc-sync/post \
        -H "Host: athena.hermes.local" \
        -H "Content-Type: text/plain" \
        -d "msg-$i" > /dev/null
    done
    END=$(date +%s)

    DURATION=$((END - START))
    RATE=$(({{COUNT}} / DURATION))
    echo "✅ Sent {{COUNT}} messages in ${DURATION}s ($RATE msg/s)"
```

**Peer Discovery:**

```justfile
# Check peer discovery via logs
test-peer-discovery:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking peer discovery..."

    CONNECTED=$(docker logs hermes-node1 2>&1 | \
      grep -c "Connected to bootstrap peer" || true)

    if [ "$CONNECTED" -ge 2 ]; then
      echo "✅ Node 1 discovered $CONNECTED peers"
    else
      echo "❌ Peer discovery incomplete ($CONNECTED/2)"
      exit 1
    fi
```

### Adding Your Own Tests

1. Open `justfile` in your editor
2. Add your test command after line 1400 (after existing tests)
3. Use this template:

```justfile
# Description of what this tests
test-your-feature:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Testing your feature..."

    # Your test logic here

    echo "✅ Test passed"
```

1. Run with: `just test-your-feature`

**Best practices:**

* Use `set -euo pipefail` for fail-fast behavior
* Add descriptive echo statements for progress
* Return non-zero exit code on failure
* Add sleep delays for async operations (mesh formation, propagation)
* Use `docker logs` or `docker compose logs` to verify behavior

---

### Fast Restart (When Binary Unchanged)

```bash
docker compose up -d             # Start without rebuilding
docker compose restart           # Restart without rebuilding
```

> **Note:** `just start` always rebuilds Docker images.
> Use `docker compose` directly to skip rebuilds.

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
Gossipsub uses `mesh_n=6` by default.
With fewer nodes, you get "Mesh low" warnings and incomplete propagation.
With 6 nodes, each connects to 5 others forming a complete mesh.

**Network Topology:**

* 6 nodes in full mesh (15 bidirectional connections)
* IPs: 172.20.0.10 through 172.20.0.15
* HTTP ports: 7878, 7880, 7882, 7884, 7886, 7888
* Persistent peer IDs stored in Docker volumes

**What happens in a test:**

1. Node 1 receives HTTP POST → publishes to PubSub topic "documents.new"
2. Gossipsub propagates message through mesh
3. All other nodes (2-6) receive and validate the message
4. Test verifies propagation by checking logs

**Behind `just quickstart`:**

* Runs `just start-ci` to build and start the mesh
* Runs `just test-pubsub-propagation` to verify propagation
* Shows success/failure with diagnostics

---

## Prerequisites

* Docker & Docker Compose
* [Just](https://just.systems)
* Rust toolchain (for building Hermes)
* [Earthly](https://earthly.dev) (Mac/Windows only - auto-detected)

**Platform support:** Builds automatically detect your OS and use the appropriate method.
Mac/Windows users get containerized builds via Earthly (slower but works everywhere).

Check prerequisites: `just validate-prereqs`

---

## All Commands

Run `just` to see all available commands, or:

**Setup & Start:**

* `just quickstart` - Complete setup and test (first-time users)
* `just start` - Start nodes (prompts for rebuild)
* `just start-ci` - Start nodes (CI mode: always rebuilds)

**Testing:**

* `just test-pubsub-propagation` - Test message propagation (interactive)
* `just test-ci` - Full CI test suite
* `just test-bidirectional-sync` - Basic test for nodes bidirectional behavior
* `just test-late-join-sync` - Test whether a node that joins late can sync with others using keepalive

**Monitoring:**

* `just logs` - All node logs
* `just status` - Node endpoints
* `just check-connectivity` - P2P connectivity report

**Management:**

* `just stop` - Stop nodes (preserves data)
* `just restart` - Restart nodes
* `just clean` - Delete everything (⚠️ deletes peer IDs)

**Troubleshooting:**

* `just troubleshoot` - Generate diagnostics report
* `just init-bootstrap` - Reset bootstrap config (auto-runs after clean)

---

## CI/CD Pipeline

```bash
just start-ci && just test-ci && just clean
```

* Always starts from clean state
* Runs full validation suite
* Cleans up after completion

> **TODO:** Integrate this CI workflow into GitHub Actions runners for automated testing on PRs

---

## Files

* `justfile` - All commands and documentation
* `docker-compose.yml` - 6-node configuration
* `Dockerfile` - Container image
* `TROUBLESHOOTING.md` - Detailed debugging guide

---

## Architecture Details

**Network Topology:**

```text
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

* Persistent IPFS keypairs (stable peer IDs)
* Bootstrap retry logic (automatic reconnection)
* Gossipsub v1.2 PubSub protocol
* Full mesh connectivity (172.20.0.0/16 network)

---

**For detailed documentation and troubleshooting, see [`justfile`](justfile) and [`TROUBLESHOOTING.md`](TROUBLESHOOTING.md)**
