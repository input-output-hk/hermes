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

```bash
cd p2p-testing
just start          # Start 6 nodes
just test-pubsub    # Verify PubSub works
just logs           # Monitor activity
just stop           # Stop nodes
```

## Commands

Run `just` or `just help` to see all available commands.

The `justfile` is self-documenting with detailed comments and examples.

## CI

```bash
just start-ci && just test-ci && just clean
```

## Prerequisites

- Docker & Docker Compose
- [Just](https://just.systems)
- [Earthly](https://earthly.dev)

## Files

- `justfile` - All commands (READ THIS for full documentation)
- `docker-compose.yml` - 3-node configuration
- `Dockerfile` - Container image

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
