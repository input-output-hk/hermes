#!/usr/bin/env bash
# Stop Hermes Docker nodes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🛑 Stopping Hermes P2P test environment"
echo "========================================"
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

# Parse arguments
REMOVE_VOLUMES=false
if [[ "${1:-}" == "--clean" ]] || [[ "${1:-}" == "-c" ]]; then
    REMOVE_VOLUMES=true
    echo "🧹 Clean mode: Will remove volumes and data"
fi

echo "🛑 Stopping nodes..."
if [ "$REMOVE_VOLUMES" = true ]; then
    $DOCKER_COMPOSE down -v
    echo "✅ All nodes stopped and data cleaned"
else
    $DOCKER_COMPOSE down
    echo "✅ All nodes stopped (data preserved in volumes)"
    echo ""
    echo "💡 To remove all data, run: ./stop-nodes.sh --clean"
fi

echo ""
