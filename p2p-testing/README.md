# Hermes P2P Testing

3-node Docker environment for testing P2P features with persistent peer identity, bootstrap retry logic, and PubSub (Gossipsub v1.2).

## Quick Start

```bash
cd p2p-testing
just start          # Start 3 nodes
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
Node 1 (172.20.0.10) ←→ Node 2 (172.20.0.11)
         ↓ ↘              ↗ ↓
           Node 3 (172.20.0.12)
```

Each node has persistent peer ID stored in Docker volumes.

**See `justfile` for detailed documentation, troubleshooting, and examples.**
