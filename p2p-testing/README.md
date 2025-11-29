# Multi-Node Testing Infrastructure for P2P Features

Docker-based setup for testing IPFS PubSub, DHT, and P2P features across multiple Hermes nodes.

## Why Docker?

Hermes will be downloaded and run by people on their own computers across the internet. Docker simulates this by:

- **Network Isolation** - Each container = separate computer with its own IP
- **Separate IPFS Repos** - Each node has isolated `/root/.hermes/` directory
- **Realistic P2P** - Nodes communicate over virtual network, not localhost
- **Cross-Platform** - Works consistently across Mac, Linux, Windows

## Why Earthly? (Critical for Docker Compatibility)

**IMPORTANT:** The build process MUST use Earthly (containerized builds) to ensure Docker compatibility:

- **GLIBC Compatibility** - Binaries built on your host (e.g., Fedora, Ubuntu) may have different GLIBC versions than the Docker container (Debian Bookworm)
- **Consistent Builds** - Earthly builds in a container with controlled dependencies, ensuring binaries work in Docker regardless of your host OS
- **Cross-Platform** - Earthly-built binaries work on any host system (Mac, Linux, Windows)

**Never use locally-built binaries** (e.g., `cargo build`) for Docker - they may have GLIBC incompatibilities and fail to run in containers.

The scripts automatically use Earthly via `just get-local-hermes` and `just get-local-athena`.

## Prerequisites

- Docker and Docker Compose
- Just ([install](https://just.systems/man/en/))
- Earthly ([install](https://earthly.dev/get-earthly)) - used by justfile

## Quick Start

```bash
# Start 3 nodes in Docker
./p2p-testing/start-nodes.sh

# Test connectivity
./p2p-testing/test-pubsub.sh

# View logs
docker compose -f p2p-testing/docker-compose.yml logs -f

# Stop nodes
./p2p-testing/stop-nodes.sh

# Stop and clean data
./p2p-testing/stop-nodes.sh --clean
```

## How It Works

### Build Process

1. **Justfile orchestrates** the build (leverages existing parallel logic):
   - `just get-local-hermes` → **Earthly containerized build** (GLIBC-compatible)
   - `just get-local-athena` → **Earthly builds WASM** + parallel packaging
2. **Docker copies** pre-built artifacts into lightweight runtime images
3. **Docker Compose** orchestrates 3 nodes on isolated network

**Why justfile?** Reuses proven parallel packaging logic instead of duplicating it.

**Why containerized builds?** Ensures the binary's GLIBC version matches the Docker runtime environment, preventing "GLIBC not found" errors across different host systems.

### Network Architecture

```
Docker Network (172.20.0.0/16)
├── Node 1 (172.20.0.10) → localhost:5000
├── Node 2 (172.20.0.11) → localhost:5002
└── Node 3 (172.20.0.12) → localhost:5004
```

Each node:
- Has its own IPFS repository
- Runs on isolated filesystem
- Can discover other nodes via P2P
- Exposes HTTP/IPFS ports to host

## Node Endpoints

| Node   | HTTP (Host) | HTTP (Container) | IPFS Swarm | IP Address   |
|--------|-------------|------------------|------------|--------------|
| Node 1 | 5000        | 5000             | 4001       | 172.20.0.10  |
| Node 2 | 5002        | 5000             | 4002       | 172.20.0.11  |
| Node 3 | 5004        | 5000             | 4003       | 172.20.0.12  |

## Testing P2P Features

### View Logs

```bash
cd p2p-testing

# All nodes
docker compose logs -f

# Single node
docker compose logs -f hermes-node1

# Search logs
docker compose logs | grep -i ipfs
```

### Access Node Shell

```bash
# Node 1
docker exec -it hermes-node1 /bin/bash

# Node 2
docker exec -it hermes-node2 /bin/bash

# Node 3
docker exec -it hermes-node3 /bin/bash
```

### Manual PubSub Testing

1. **Terminal 1 - Subscribe on Node 1:**
   ```bash
   docker exec -it hermes-node1 /bin/bash
   # Use Hermes IPFS commands to subscribe
   ```

2. **Terminal 2 - Subscribe on Node 2:**
   ```bash
   docker exec -it hermes-node2 /bin/bash
   # Use Hermes IPFS commands to subscribe
   ```

3. **Terminal 3 - Publish from Node 3:**
   ```bash
   docker exec -it hermes-node3 /bin/bash
   # Use Hermes IPFS commands to publish
   ```

### Network Testing

Nodes can communicate via Docker network:
```bash
# From inside node1 container
ping 172.20.0.11  # Ping node2
ping 172.20.0.12  # Ping node3
```

## Development Workflow

### Rebuild After Code Changes

```bash
cd p2p-testing

# Stop nodes
./stop-nodes.sh

# Rebuild (uses justfile)
cd ..
just get-local-hermes
just get-local-athena  # Earthly + parallel packaging

# Or restart which will detect changes
./p2p-testing/start-nodes.sh
```

### Quick Restart (no rebuild)

```bash
cd p2p-testing
./stop-nodes.sh
./start-nodes.sh  # Will use existing build
```

### Clean Start

```bash
cd p2p-testing
./stop-nodes.sh --clean  # Remove all data
./start-nodes.sh         # Fresh start
```

## Configuration

### Environment Variables

Set in `docker-compose.yml`:

- `HERMES_HTTP_PORT` - HTTP gateway port (5000 inside container)
- `HERMES_ACTIVATE_AUTH` - Enable authentication (default: true)
- `REDIRECT_ALLOWED_HOSTS` - Allowed redirect hosts
- `REDIRECT_ALLOWED_PATH_PREFIXES` - Allowed path prefixes

### Add More Nodes

Edit `docker-compose.yml`:

```yaml
hermes-node4:
  # Copy node3 config
  container_name: hermes-node4
  hostname: node4
  ports:
    - "5006:5000"  # Increment host ports
    - "4004:4001"
    - "5007:5001"
  volumes:
    - node4-data:/data
  networks:
    hermes-p2p:
      ipv4_address: 172.20.0.13  # Increment IP
```

Add volume:
```yaml
volumes:
  node4-data:
```

## Files

- `p2p-testing/Dockerfile` - Runtime image (copies Earthly artifacts)
- `p2p-testing/docker-compose.yml` - 3-node orchestration
- `p2p-testing/start-nodes.sh` - Build and start nodes
- `p2p-testing/stop-nodes.sh` - Stop nodes
- `p2p-testing/test-pubsub.sh` - Test framework
- `p2p-testing/README.md` - This file

## Troubleshooting

### Docker daemon not running

```bash
sudo systemctl start docker
```

### Earthly build fails

```bash
# Check Earthly installation
earthly --version

# Try building directly
cd hermes
earthly +build
earthly +build-athena
```

### Nodes not starting

Check logs:
```bash
cd p2p-testing
docker compose logs
```

### GLIBC errors ("GLIBC_X.XX not found")

This means the binary was built locally instead of with Earthly. **Solution:**

```bash
# Stop nodes
./p2p-testing/stop-nodes.sh --clean

# Rebuild with Earthly (containerized build)
just get-local-hermes
just get-local-athena

# Restart nodes
./p2p-testing/start-nodes.sh
```

The start script will prompt you to rebuild with Earthly if existing artifacts are found.

### Port conflicts

Check if ports are in use:
```bash
lsof -i :5000,5002,5004
```

### Network issues

Reset Docker:
```bash
cd p2p-testing
./stop-nodes.sh --clean
docker network prune
./start-nodes.sh
```

## Related Issues

- Issue #704: Multi-Node Testing Infrastructure for P2P Features
