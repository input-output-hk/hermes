# Hermes P2P Testing Infrastructure

Multi-node Docker environment for testing Hermes P2P features including IPFS connectivity, bootstrap nodes, and PubSub messaging.

## Features

- **3-Node Isolated Network**: Simulates distributed deployment on 172.20.0.0/16
- **Persistent Peer Identity**: Stable peer IDs across restarts via keypair persistence
- **Bootstrap Retry Logic**: Automatic reconnection with exponential backoff
- **PubSub Support**: Gossipsub v1.2 protocol for message propagation
- **CI-Ready**: Non-interactive commands for automated testing
- **Justfile Commands**: Clean, organized command interface

## Why Docker?

Hermes will be downloaded and run by people on their own computers across the internet. Docker simulates this by:

- **Network Isolation** - Each container = separate computer with its own IP
- **Separate IPFS Repos** - Each node has isolated `~/.hermes/` directory
- **Realistic P2P** - Nodes communicate over virtual network, not localhost
- **Cross-Platform** - Works consistently across Mac, Linux, Windows

## Why Earthly? (Critical for Docker Compatibility)

**IMPORTANT:** The build process uses Earthly (containerized builds) to ensure Docker compatibility:

- **GLIBC Compatibility** - Binaries built on your host may have different GLIBC versions than the Docker container
- **Consistent Builds** - Earthly builds in a controlled environment, ensuring binaries work in Docker
- **Cross-Platform** - Earthly-built binaries work on any host system

**Never use locally-built binaries** (`cargo build`) for Docker - they may have GLIBC incompatibilities.

## Prerequisites

- Docker and Docker Compose
- Just ([install](https://just.systems/man/en/))
- Earthly ([install](https://earthly.dev/get-earthly)) - used by justfile

## Quick Start

```bash
# Start the test environment
cd p2p-testing
just start

# Check status
just status

# Test PubSub functionality
just test-pubsub

# View logs
just logs

# Stop nodes
just stop
```

## Architecture

### Network Topology

```
172.20.0.0/16 Docker Network
├── Node 1 (172.20.0.10) - Bootstrap peer for nodes 2 & 3
├── Node 2 (172.20.0.11) - Bootstrap peer for nodes 1 & 3
└── Node 3 (172.20.0.12) - Bootstrap peer for nodes 1 & 2
```

### Persistent Peer IDs

Each node stores its libp2p keypair at `~/.hermes/ipfs/keypair`, ensuring stable peer IDs across restarts:

- **Node 1**: `12D3KooWBxass2EcccdN5FzC2e6er3uyuYJkTchMX11iKWaJ9aj1`
- **Node 2**: `12D3KooWBkdsbenzeixaTmEXdmpf5P2pXtyvm4Qb4sGZJ1Wi4BUB`
- **Node 3**: `12D3KooWA5tEBmYCXQfmAdt5zZonBouBt2KBN6umaUtdQVY39GPK`

### Port Mapping

| Node | HTTP | IPFS P2P | IPFS API | IP Address   |
|------|------|----------|----------|--------------|
| 1    | 5000 | 4001     | 5001     | 172.20.0.10 |
| 2    | 5002 | 4002     | 5003     | 172.20.0.11 |
| 3    | 5004 | 4003     | 5005     | 172.20.0.12 |

## Commands

### Setup & Build

```bash
just build              # Build Hermes binary (fast local build)
just build-all          # Build Hermes + Athena WASM
just build-images       # Build Docker images
just build-ci           # Build everything (CI mode, no prompts)
```

### Node Management

```bash
just start              # Start nodes (prompts to rebuild if artifacts exist)
just start-ci           # Start nodes (CI mode, always rebuilds)
just stop               # Stop nodes (preserve data in volumes)
just clean              # Stop nodes and remove all data
just restart            # Stop and start (preserves data)
just reset              # Clean everything and start fresh
```

### Monitoring

```bash
just status             # Show node status and endpoints
just logs               # Follow logs from all nodes
just logs-node 1        # Follow logs from specific node (1, 2, or 3)
just check-connectivity # Check P2P connection status
```

### Testing

```bash
just test               # Run integration tests on all nodes
just test-node1         # Run tests on node 1
just test-node2         # Run tests on node 2
just test-node3         # Run tests on node 3
just test-pubsub        # Verify PubSub infrastructure
just test-ci            # Full CI test suite (status + connectivity + pubsub)
```

## CI Integration

For automated testing in CI pipelines:

```bash
# Full CI workflow
just start-ci           # Start nodes (no prompts, always clean build)
just test-ci            # Run full test suite
just clean              # Clean up
```

The `test-ci` command verifies:
1. All 3 nodes are running
2. Nodes are listening on port 4001
3. Bootstrap connections succeeded
4. Gossipsub protocol is active
5. Peer connections are established

## PubSub Testing

The `test-pubsub` command verifies the PubSub infrastructure is ready by checking:

1. **Gossipsub Protocol**: Verifies Gossipsub v1.2 is active on all nodes
2. **Peer Connections**: Confirms nodes are connected (expects 6 peer connections)
3. **Bootstrap Status**: Checks bootstrap retry logic succeeded
4. **Infrastructure**: Validates listening addresses and protocol readiness

To test actual message propagation, use the WASM integration tests:

```bash
just test               # Runs integration tests including PubSub subscribe/publish
```

## Bootstrap Retry Logic

Nodes automatically retry failed bootstrap connections:

- **Retry Interval**: 10 seconds
- **Max Retries**: 10 attempts
- **Implementation**: `hermes/bin/src/ipfs/mod.rs:99-128`

Logs show retry progress:
```
Bootstrap retry 1/10: attempting 2 peer(s)
✓ Bootstrap retry succeeded: /ip4/172.20.0.11/tcp/4001/p2p/...
✓ All bootstrap peers connected
```

## Key Implementation Details

### Persistent Keypair Storage

**Location**: `hermes/bin/src/ipfs/mod.rs:34-57`

The `load_or_generate_keypair()` function:
- Checks for existing keypair at `~/.hermes/ipfs/keypair`
- Loads existing keypair using protobuf encoding
- Generates new Ed25519 keypair if none exists
- Logs peer ID for verification

### Bootstrap Retry

**Location**: `hermes/bin/src/ipfs/mod.rs:99-128`

The `retry_bootstrap_connections()` function:
- Spawns async task for failed bootstrap peers
- Retries every 10 seconds up to 10 times
- Removes successful connections from retry list
- Logs final status (success or failure)

### Listening Address Configuration

**Location**: `hermes/bin/src/ipfs/mod.rs:202-211`

After node initialization, `add_listening_address()` is called to bind to port 4001:
- Parses multiaddr `/ip4/0.0.0.0/tcp/4001`
- Configures node to listen on all interfaces
- Logs success or failure with clear error messages

## Troubleshooting

### Nodes not connecting

```bash
# Check if nodes are listening on port 4001
just check-connectivity

# View bootstrap logs
just logs | grep bootstrap

# Check for connection errors
just logs | grep "Connection refused"
```

### Docker network conflicts

If you see "Pool overlaps with other one":

```bash
# List networks using 172.20.x.x
docker network ls --format '{{.Name}}' | xargs -I {} sh -c 'docker network inspect {} --format "{{.Name}}: {{range .IPAM.Config}}{{.Subnet}}{{end}}" 2>/dev/null | grep 172.20'

# Remove conflicting network (if not in use)
docker network rm <network-name>

# Restart nodes
just restart
```

### Stale containers

```bash
# Remove old containers with same names
docker rm hermes-node1 hermes-node2 hermes-node3

# Or use clean mode
just clean
just start
```

### Rebuilding after code changes

```bash
# Rebuild Hermes binary
just build

# Rebuild Docker images
just build-images

# Restart nodes with new binary
just restart
```

### GLIBC errors ("GLIBC_X.XX not found")

This means the binary was built locally instead of with Earthly. **Solution:**

```bash
# Clean everything
just clean

# Rebuild with Earthly
just build-all

# Restart
just start
```

## Development Workflow

### Making changes to P2P logic

1. Modify Rust code in `hermes/bin/src/ipfs/mod.rs`
2. Rebuild: `just build`
3. Rebuild images: `just build-images`
4. Restart: `just restart`
5. Test: `just test-pubsub`

### Testing new bootstrap strategies

1. Update `IPFS_BOOTSTRAP_PEERS` in `docker-compose.yml`
2. Restart: `just reset` (clean start recommended)
3. Monitor: `just logs | grep bootstrap`

### Adding new nodes

1. Add service in `docker-compose.yml`
2. Assign static IP in `172.20.0.0/16` range
3. Update bootstrap peer lists for all nodes
4. Update port mappings in README

## Files

- `justfile` - All commands and recipes (USE THIS)
- `docker-compose.yml` - 3-node Docker configuration
- `Dockerfile` - Hermes container image
- `README.md` - This file

### Legacy Scripts (Deprecated)

The following shell scripts have been replaced by the justfile and should not be used:
- `start-nodes.sh` → `just start`
- `stop-nodes.sh` → `just stop` / `just clean`
- `test-pubsub.sh` → `just test-pubsub`
- `connect-nodes.sh` → No longer needed (auto-bootstrap)
- `initialize-p2p.sh` → No longer needed (auto-bootstrap)

These scripts are kept for reference only.

## Related Documentation

- [Hermes IPFS Module](../hermes/bin/src/ipfs/mod.rs)
- [Docker Compose Config](./docker-compose.yml)
- [WASM Integration Tests](../wasm/integration-test/ipfs/src/lib.rs)
- [Catalyst Libs - hermes-ipfs](https://github.com/input-output-hk/catalyst-libs/tree/feat/hermes-ipfs-persistent-keypair)

## Related Issues

- Issue #704: Multi-Node Testing Infrastructure for P2P Features
