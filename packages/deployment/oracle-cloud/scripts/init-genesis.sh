#!/bin/bash
# Jeju L2 Genesis Initialization Script
#
# This script initializes the rollup.json with actual L1 genesis hash
# and sets up proper timestamps. Run this AFTER L1 is started but BEFORE
# starting op-node.
#
# Usage:
#   ./init-genesis.sh [L1_RPC_URL]
#   Default L1_RPC_URL: http://localhost:8545

set -e

L1_RPC="${1:-http://localhost:8545}"
CONFIG_DIR="$(dirname "$0")/../docker/config"
ROLLUP_JSON="$CONFIG_DIR/rollup.json"
GENESIS_JSON="$CONFIG_DIR/genesis.json"

echo "=== Jeju L2 Genesis Initialization ==="
echo "L1 RPC: $L1_RPC"
echo ""

# Wait for L1 to be ready
echo "Waiting for L1 to be ready..."
for i in {1..30}; do
    if curl -s "$L1_RPC" -X POST -H 'Content-Type: application/json' \
        --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null 2>&1; then
        echo "L1 is ready!"
        break
    fi
    echo "Waiting... ($i/30)"
    sleep 2
done

# Get L1 genesis block hash
echo ""
echo "Fetching L1 genesis block hash..."
L1_HASH=$(curl -s "$L1_RPC" -X POST -H 'Content-Type: application/json' \
    --data '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["0x0",false],"id":1}' \
    | grep -o '"hash":"0x[a-fA-F0-9]*"' | head -1 | cut -d'"' -f4)

if [ -z "$L1_HASH" ] || [ "$L1_HASH" = "null" ]; then
    echo "ERROR: Could not fetch L1 genesis hash"
    exit 1
fi
echo "L1 Genesis Hash: $L1_HASH"

# Get current timestamp
CURRENT_TIME=$(date +%s)
echo "L2 Genesis Time: $CURRENT_TIME"

# Calculate L2 genesis hash (use L1 hash as placeholder since we haven't initialized op-geth yet)
# In production, you'd initialize op-geth first and get the actual L2 hash
L2_HASH="0x0000000000000000000000000000000000000000000000000000000000000000"

# Update rollup.json
echo ""
echo "Updating rollup.json..."

# Use jq if available, otherwise use sed
if command -v jq &> /dev/null; then
    jq --arg l1hash "$L1_HASH" --arg l2time "$CURRENT_TIME" '
        .genesis.l1.hash = $l1hash |
        .genesis.l2_time = ($l2time | tonumber)
    ' "$ROLLUP_JSON" > "$ROLLUP_JSON.tmp" && mv "$ROLLUP_JSON.tmp" "$ROLLUP_JSON"
else
    # Fallback to sed
    sed -i.bak "s|\"hash\": \"0x0000000000000000000000000000000000000000000000000000000000000000\"|\"hash\": \"$L1_HASH\"|" "$ROLLUP_JSON"
    sed -i.bak "s|\"l2_time\": 0|\"l2_time\": $CURRENT_TIME|" "$ROLLUP_JSON"
    rm -f "$ROLLUP_JSON.bak"
fi

# Update genesis.json timestamp
echo "Updating genesis.json timestamp..."
TIMESTAMP_HEX=$(printf '0x%x' $CURRENT_TIME)

if command -v jq &> /dev/null; then
    jq --arg ts "$TIMESTAMP_HEX" '.timestamp = $ts' "$GENESIS_JSON" > "$GENESIS_JSON.tmp" && mv "$GENESIS_JSON.tmp" "$GENESIS_JSON"
else
    sed -i.bak "s|\"timestamp\": \"0x0\"|\"timestamp\": \"$TIMESTAMP_HEX\"|" "$GENESIS_JSON"
    rm -f "$GENESIS_JSON.bak"
fi

echo ""
echo "=== Configuration Updated ==="
echo "L1 Hash: $L1_HASH"
echo "L2 Time: $CURRENT_TIME ($TIMESTAMP_HEX)"
echo ""
echo "IMPORTANT: After starting op-geth, you may need to update the L2 hash in rollup.json"
echo "Run: curl -s localhost:9545 -X POST -H 'Content-Type: application/json' \\"
echo "  --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"0x0\",false],\"id\":1}'"
echo ""
echo "Genesis initialization complete!"
