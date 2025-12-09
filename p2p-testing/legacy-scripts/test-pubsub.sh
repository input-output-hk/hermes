#!/usr/bin/env bash
# Test IPFS PubSub across Docker nodes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🧪 Testing IPFS PubSub across Hermes nodes (Docker)"
echo "===================================================="
echo ""

# Check if docker compose is available
if docker compose version &> /dev/null 2>&1; then
    DOCKER_COMPOSE="docker compose"
elif command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
else
    echo "❌ Error: docker compose not found"
    exit 1
fi

cd "$SCRIPT_DIR"

# Check if nodes are running
if ! $DOCKER_COMPOSE ps | grep -q "hermes-node1.*Up\|hermes-node1.*running"; then
    echo "❌ Error: Nodes are not running"
    echo "   Start nodes with: ./start-nodes.sh"
    exit 1
fi

echo "📊 Checking node status..."
$DOCKER_COMPOSE ps

echo ""
echo "🔌 Testing HTTP connectivity..."
for i in 1 2 3; do
    port=$((5000 + (i-1)*2))
    if curl -s -o /dev/null -w "%{http_code}" "http://localhost:$port/" 2>/dev/null | grep -q "200\|404"; then
        echo "   ✅ Node $i (port $port) is reachable"
    else
        echo "   ⚠️  Node $i (port $port) not responding"
    fi
done

echo ""
echo "🌐 Network Configuration:"
echo "   Nodes are on isolated Docker network: 172.20.0.0/16"
echo "   - Node 1: 172.20.0.10"
echo "   - Node 2: 172.20.0.11"
echo "   - Node 3: 172.20.0.12"
echo ""

echo "📡 IPFS PubSub Test Plan:"
echo "   1. Subscribe to topic on Node 1 and Node 2"
echo "   2. Publish message from Node 3"
echo "   3. Verify Nodes 1 and 2 receive the message"
echo ""

echo "💡 To test PubSub manually:"
echo "   # Terminal 1 - Node 1 shell"
echo "   docker exec -it hermes-node1 /bin/bash"
echo ""
echo "   # Terminal 2 - Node 2 shell"
echo "   docker exec -it hermes-node2 /bin/bash"
echo ""
echo "   # Terminal 3 - Node 3 shell"
echo "   docker exec -it hermes-node3 /bin/bash"
echo ""

echo "📋 Recent logs from nodes:"
echo "----------------------------------------"
$DOCKER_COMPOSE logs --tail=10 | grep -i "ipfs\|pubsub\|p2p\|bootstrap" || echo "   (No P2P-related logs found yet)"

echo ""
echo "✅ P2P test framework ready!"
echo ""
echo "💡 Next steps:"
echo "   - View logs:     $DOCKER_COMPOSE logs -f"
echo "   - Node shell:    docker exec -it hermes-node1 /bin/bash"
echo "   - Stop nodes:    ./stop-nodes.sh"
echo ""
