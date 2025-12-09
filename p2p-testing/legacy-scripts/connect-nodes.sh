#!/usr/bin/env bash
# Manually connect the three Hermes nodes using current peer IDs

set -euo pipefail

echo "🔗 Connecting Hermes P2P nodes..."
echo ""

# Get current peer IDs from running containers
echo "📋 Extracting peer IDs from running nodes..."
NODE1_ID=$(docker compose logs hermes-node1 2>&1 | grep "starting with peer id" | tail -1 | grep -oE '12D3[a-zA-Z0-9]+')
NODE2_ID=$(docker compose logs hermes-node2 2>&1 | grep "starting with peer id" | tail -1 | grep -oE '12D3[a-zA-Z0-9]+')
NODE3_ID=$(docker compose logs hermes-node3 2>&1 | grep "starting with peer id" | tail -1 | grep -oE '12D3[a-zA-Z0-9]+')

echo "  Node 1 (172.20.0.10): $NODE1_ID"
echo "  Node 2 (172.20.0.11): $NODE2_ID"
echo "  Node 3 (172.20.0.12): $NODE3_ID"
echo ""

# Build multiaddrs
NODE1_ADDR="/ip4/172.20.0.10/tcp/4001/p2p/$NODE1_ID"
NODE2_ADDR="/ip4/172.20.0.11/tcp/4001/p2p/$NODE2_ID"
NODE3_ADDR="/ip4/172.20.0.12/tcp/4001/p2p/$NODE3_ID"

echo "🔌 Connecting nodes..."

# Try using IPFS API (port 5001, 5003, 5005)
# Node 1 connects to Node 2 and 3
echo "  Node 1 → Node 2, Node 3"
curl -X POST "http://localhost:5001/api/v0/swarm/connect?arg=$NODE2_ADDR" 2>/dev/null || echo "    (connection may already exist or API not available)"
curl -X POST "http://localhost:5001/api/v0/swarm/connect?arg=$NODE3_ADDR" 2>/dev/null || echo "    (connection may already exist or API not available)"

# Node 2 connects to Node 1 and 3
echo "  Node 2 → Node 1, Node 3"
curl -X POST "http://localhost:5003/api/v0/swarm/connect?arg=$NODE1_ADDR" 2>/dev/null || echo "    (connection may already exist or API not available)"
curl -X POST "http://localhost:5003/api/v0/swarm/connect?arg=$NODE3_ADDR" 2>/dev/null || echo "    (connection may already exist or API not available)"

# Node 3 connects to Node 1 and 2
echo "  Node 3 → Node 1, Node 2"
curl -X POST "http://localhost:5005/api/v0/swarm/connect?arg=$NODE1_ADDR" 2>/dev/null || echo "    (connection may already exist or API not available)"
curl -X POST "http://localhost:5005/api/v0/swarm/connect?arg=$NODE2_ADDR" 2>/dev/null || echo "    (connection may already exist or API not available)"

echo ""
echo "✅ Connection attempts complete!"
echo ""
echo "🧪 To test pubsub, use the Hermes IPFS API in your application,"
echo "   or check the connection status:"
echo ""
echo "   # Check Node 1 peers:"
echo "   curl -s http://localhost:5001/api/v0/swarm/peers | jq"
echo ""
echo "   # Check Node 2 peers:"
echo "   curl -s http://localhost:5003/api/v0/swarm/peers | jq"
echo ""
echo "   # Check Node 3 peers:"
echo "   curl -s http://localhost:5005/api/v0/swarm/peers | jq"
echo ""
