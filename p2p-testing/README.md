# Hermes P2P Testing

6-node Docker environment for testing P2P features with persistent peer identity, bootstrap retry logic, and PubSub (Gossipsub v1.2).

## Why 6 Nodes?

Gossipsub (libp2p's PubSub protocol) uses `mesh_n=6` by default, meaning each node expects to maintain connections to 6 peers in its mesh for optimal message propagation. With fewer than 6 nodes:
- Nodes log "Mesh low" warnings
- PubSub publish operations block waiting for the mesh to reach target size
- End-to-end message propagation doesn't complete

**Alternatives considered:**
- Fork `rust-ipfs` to add small mesh configuration (mesh_n=2 for 3-node setups)
- Requires modifying PubsubConfig and builder methods
- 6-node setup avoids forking external dependencies

## Quick Start

### First Time Setup
```bash
cd p2p-testing
just quickstart     # Does everything: build, start, test, verify
```

### Daily Workflow
```
┌─────────────┐     ┌─────────────┐     ┌──────────────┐     ┌─────────┐     ┌──────┐
│ just start  │ →   │  dashboard  │ →   │ test-pubsub- │ →   │  logs   │ →   │ stop │
│             │     │             │     │ propagation  │     │         │     │      │
└─────────────┘     └─────────────┘     └──────────────┘     └─────────┘     └──────┘
```

```bash
just start              # Start 6 nodes (waits for mesh formation)
just dashboard          # Check status
just test-pubsub-propagation  # Test message propagation with visualization
just logs               # Monitor activity
just stop               # Stop nodes (preserves peer IDs)
```

### When Things Go Wrong
```bash
just health-check       # Quick diagnostic
just troubleshoot       # Full diagnostic report
```

### Complete Reset
```bash
just clean              # Remove all data and volumes
just quickstart         # Start fresh from scratch
```
**⚠️ IMPORTANT:** After `just clean`, peer IDs are regenerated automatically on next start.

## Essential Commands

**Getting Started:**
- `just quickstart` - Complete end-to-end setup and test (recommended for first-time users)
- `just validate-prereqs` - Check all prerequisites before running
- `just start` - Start 6 nodes (interactive mode: prompts if rebuild needed)
- `just start-ci` - Start nodes (CI mode: non-interactive, always rebuilds from clean state)

**Testing & Monitoring:**
- `just test-pubsub-propagation` - Test end-to-end message propagation
- `just health-check` - Comprehensive system health check
- `just dashboard` - Live status display with node info
- `just logs` - View all logs
- `just status` - Show node endpoints

**Troubleshooting:**
- `just troubleshoot` - Generate full diagnostics report (saves to file)
- `just check-connectivity` - Check P2P connectivity
- `just init-bootstrap` - Reset bootstrap configuration (auto-runs on first start)

**Management:**
- `just stop` - Stop nodes (preserves data)
- `just restart` - Restart all nodes
- `just clean` - Stop and remove all data (**deletes peer IDs**)

Run `just` or `just help` to see all available commands.

## CI/CD Pipeline

**Complete CI workflow:**
```bash
just start-ci && just test-ci && just clean
```

- `start-ci` - Always rebuilds Docker images from clean state (no prompts)
- `test-ci` - Runs full validation suite (status + connectivity + pubsub)
- `clean` - Removes all containers and volumes

**Note:** CI mode always starts from a clean state to ensure reproducibility.

## Prerequisites

- Docker & Docker Compose
- [Just](https://just.systems)
- [Earthly](https://earthly.dev)

## What to Do If Tests Fail

If `just test-pubsub-propagation` fails, follow these steps:

### 1. Quick Health Check
```bash
just health-check
```
This checks all nodes, peer connections, Gossipsub, and bootstrap status.

### 2. Common Fixes

**Mesh Still Forming** (most common issue):
```bash
# Wait for mesh to fully form
sleep 30 && just test-pubsub-propagation
```

**Port Conflicts:**
```bash
just stop
# Check what's using ports: lsof -i :5000
just start
```

**Complete Reset:**
```bash
just clean && just quickstart
```

### 3. Generate Full Diagnostics
```bash
just troubleshoot  # Creates p2p-troubleshoot-TIMESTAMP.txt
```

### 4. View Detailed Logs
```bash
just logs | grep -E '(RECEIVED|gossipsub|bootstrap)'
```

### 5. Still Stuck?
See `TROUBLESHOOTING.md` for comprehensive debugging guide with solutions for:
- Nodes won't start
- Partial propagation
- Docker build failures
- Bootstrap initialization issues
- Performance problems

## Files

- `justfile` - All commands (READ THIS for full documentation)
- `docker-compose.yml` - 6-node configuration
- `Dockerfile` - Container image
- `TROUBLESHOOTING.md` - Comprehensive debugging guide

## Features

- Persistent IPFS keypairs (stable peer IDs)
- Bootstrap retry logic (automatic reconnection)
- Gossipsub v1.2 PubSub protocol
- Full mesh connectivity (172.20.0.0/16 network)

## Architecture

```
        Node 1 (172.20.0.10)
       /  |  \
      /   |   \
     /    |    \
Node 2   Node 4  Node 6
(.11)    (.13)   (.15)
  \      |      /
   \     |     /
    \    |    /
     Node 3  Node 5
     (.12)   (.14)
```

6 nodes in full mesh topology. Each node connects to all others.
Each node has persistent peer ID stored in Docker volumes.

**See `justfile` for detailed documentation, troubleshooting, and examples.**
