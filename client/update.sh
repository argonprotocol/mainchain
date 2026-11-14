#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -eo pipefail
set -x

BASEDIR=$(dirname "$0")
PIPE="/tmp/argon-node-output"
RPC_PORT=9900
RPC_HTTP_URL="http://127.0.0.1:${RPC_PORT}"
RPC_WS_URL="ws://127.0.0.1:${RPC_PORT}"

rm -f "$PIPE"
mkfifo "$PIPE"

# Function to clean up before exit
cleanup() {
    echo "Cleaning up..."
    rm -f "$PIPE"
    kill "$argon_PID" 2>/dev/null || true
    wait "$argon_PID" 2>/dev/null || true
}

trap cleanup EXIT SIGHUP SIGINT SIGTERM

"${CARGO_TARGET_DIR:-$BASEDIR/../target}/debug/argon-node" --tmp --no-mdns --no-prometheus --chain=meta --rpc-port="${RPC_PORT}" --compute-miners=0 --bitcoin-rpc-url="http://127.0.0.1:18443" > "$PIPE" 2>&1 &
set +x
argon_PID=$!

while IFS= read -r line; do
    echo "$line"
    if curl -sf -H "Content-Type: application/json" \
        -d '{"id":"1", "jsonrpc":"2.0", "method": "system_health", "params":[]}' \
        "${RPC_HTTP_URL}" >/dev/null; then
        echo "Detected JSON-RPC server startup."
        break
    fi
done <"$PIPE"
set -x

if ! command -v subxt &> /dev/null; then
    cargo install -f subxt-cli
fi

subxt metadata --url "${RPC_WS_URL}" --version 15 --format bytes > "$BASEDIR/metadata.scale"

# get runtime spec version
curl -H "Content-Type: application/json" -d '{"id":"1", "jsonrpc":"2.0", "method": "state_getRuntimeVersion", "params":[]}' "${RPC_HTTP_URL}" | jq -r '.result' > "$BASEDIR/nodejs/runtime_version.json"

# Cleanup and exit (this will be called automatically via trap)
cleanup
