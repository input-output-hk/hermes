#!/usr/bin/env bash
# Start multi-node Hermes P2P test environment using Docker

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

# Use justfile recipes (leverages Earthly + parallel packaging)
echo "🔨 Building with justfile (Earthly + parallel packaging)..."
echo ""

if [ -f "hermes/target/release/hermes" ] && [ -f "hermes/apps/athena/athena.happ" ]; then
    echo "📦 Found existing build artifacts"
    read -p "   Rebuild? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "   🔄 Rebuilding..."
        just get-local-hermes    # Earthly build
        just get-local-athena    # Earthly build + parallel packaging
        echo "✅ Build complete!"
    else
        echo "   ⏭️  Using existing artifacts"
    fi
else
    echo "🏗️  Building from scratch..."
    just get-local-hermes    # Earthly build
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
$DOCKER_COMPOSE up -d

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
