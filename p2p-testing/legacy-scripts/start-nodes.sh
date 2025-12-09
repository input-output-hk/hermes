#!/usr/bin/env bash
# Start multi-node Hermes P2P test environment using Docker
#
# IMPORTANT: This script uses Earthly (containerized builds) to ensure
# cross-platform compatibility. Binaries are built in a controlled environment
# with GLIBC versions matching the Docker container (Debian Bookworm).
#
# Never use locally-built binaries (cargo build) as they may have GLIBC
# incompatibilities with Docker and fail to run across different host systems.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🚀 Starting Hermes multi-node P2P test environment (Docker)"
echo "============================================================"
echo ""

# Check if docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Error: docker not found"
    echo "   Please install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if docker compose is available
if docker compose version &> /dev/null 2>&1; then
    DOCKER_COMPOSE="docker compose"
elif command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
else
    echo "❌ Error: docker compose not found"
    echo "   Please install Docker Compose"
    exit 1
fi

# Check if just is available
if ! command -v just &> /dev/null; then
    echo "❌ Error: just not found"
    echo "   Please install just: https://just.systems/man/en/"
    exit 1
fi

cd "$PROJECT_ROOT"

# Use justfile recipes with Earthly for cross-platform compatibility
echo "🔨 Building with Earthly (containerized build for cross-platform compatibility)..."
echo ""

if [ -f "hermes/target/release/hermes" ] && [ -f "hermes/apps/athena/athena.happ" ]; then
    echo "📦 Found existing build artifacts"
    read -p "   Rebuild with Earthly? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "   🔄 Rebuilding with Earthly (ensures Docker compatibility)..."
        just get-local-hermes    # Earthly containerized build - GLIBC compatible
        just get-local-athena    # Earthly build + parallel packaging
        echo "✅ Build complete!"
    else
        echo "   ⏭️  Using existing artifacts"
        echo "   ⚠️  Note: If Docker fails with GLIBC errors, rebuild with Earthly (y above)"
    fi
else
    echo "🏗️  Building from scratch with Earthly..."
    echo "   This ensures the binary works in Docker regardless of your host OS"
    just get-local-hermes    # Earthly containerized build - GLIBC compatible
    just get-local-athena    # Earthly build + parallel packaging
    echo "✅ Build complete!"
fi

echo ""

# Build Docker images
cd "$SCRIPT_DIR"
echo "🐳 Building Docker images..."
$DOCKER_COMPOSE build

echo ""
echo "🌐 Starting 3 Hermes nodes in Docker..."

# Try to start the nodes
if ! $DOCKER_COMPOSE up -d 2>&1 | tee /tmp/docker-compose-output.log; then
    # Check if the error is due to network overlap
    if grep -q "Pool overlaps with other one" /tmp/docker-compose-output.log; then
        echo ""
        echo "❌ Error: Docker network conflict detected!"
        echo ""
        echo "The p2p-testing environment uses subnet 172.20.0.0/16, but another"
        echo "network is already using this address space."
        echo ""
        echo "To fix this, find and remove the conflicting network:"
        echo ""
        echo "  1. List networks using this subnet:"
        echo "     docker network ls --format '{{.Name}}' | xargs -I {} sh -c 'docker network inspect {} --format \"{{.Name}}: {{range .IPAM.Config}}{{.Subnet}}{{end}}\" 2>/dev/null | grep 172.20'"
        echo ""
        echo "  2. Remove the conflicting network (if not in use):"
        echo "     docker network rm <network-name>"
        echo ""
        echo "  3. Re-run this script"
        echo ""
        rm -f /tmp/docker-compose-output.log
        exit 1
    # Check if the error is due to container name conflict
    elif grep -q "is already in use by container" /tmp/docker-compose-output.log; then
        echo ""
        echo "❌ Error: Container name conflict detected!"
        echo ""
        echo "Old containers with the same names are still present from a previous run."
        echo ""
        echo "To fix this, remove the old containers:"
        echo ""
        echo "  docker rm hermes-node1 hermes-node2 hermes-node3"
        echo ""
        echo "Or use the stop script with clean flag:"
        echo ""
        echo "  ./stop-nodes.sh --clean"
        echo ""
        rm -f /tmp/docker-compose-output.log
        exit 1
    else
        # Different error - show the output
        cat /tmp/docker-compose-output.log
        rm -f /tmp/docker-compose-output.log
        exit 1
    fi
fi

rm -f /tmp/docker-compose-output.log

echo ""
echo "⏳ Waiting for nodes to initialize..."
sleep 5

echo ""
echo "📊 Node Status:"
$DOCKER_COMPOSE ps

echo ""
echo "✅ Multi-node P2P environment is ready!"
echo ""
echo "📡 Node Endpoints (from host):"
echo "   Node 1: http://localhost:5000 (IPFS: 4001, API: 5001) [172.20.0.10]"
echo "   Node 2: http://localhost:5002 (IPFS: 4002, API: 5003) [172.20.0.11]"
echo "   Node 3: http://localhost:5004 (IPFS: 4003, API: 5005) [172.20.0.12]"
echo ""
echo "💡 Useful commands:"
echo "   View logs:        $DOCKER_COMPOSE logs -f"
echo "   View node1 logs:  $DOCKER_COMPOSE logs -f hermes-node1"
echo "   Stop nodes:       ./stop-nodes.sh"
echo "   Test PubSub:      ./test-pubsub.sh"
echo "   Node shell:       docker exec -it hermes-node1 /bin/bash"
echo ""
