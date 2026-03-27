#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e
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

"$BASEDIR/../target/debug/argon-node" --tmp --no-mdns --no-prometheus --chain=meta --rpc-port="${RPC_PORT}" --compute-miners=0 --bitcoin-rpc-url="http://127.0.0.1:18443" > "$PIPE" 2>&1 &
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

# Install subxt if not already installed
if ! command -v subxt &> /dev/null; then
    cargo install -f subxt-cli
fi

subxt codegen --url "${RPC_WS_URL}" --derive Clone \
  --derive-for-type bounded_collections::bounded_vec::BoundedVec=serde::Serialize \
  --attributes-for-type bounded_collections::bounded_vec::BoundedVec="#[serde(transparent)]" \
  --substitute-type primitive_types::H256=crate::types::H256 \
  --substitute-type sp_core::crypto::AccountId32=crate::types::AccountId32 \
   | rustfmt > "$BASEDIR/src/spec.rs"

curl -H "Content-Type: application/json" -d '{"id":"1", "jsonrpc":"2.0", "method": "state_getMetadata", "params":[]}' "${RPC_HTTP_URL}" > "$BASEDIR/nodejs/metadata.json"

# get runtime spec version
curl -H "Content-Type: application/json" -d '{"id":"1", "jsonrpc":"2.0", "method": "state_getRuntimeVersion", "params":[]}' "${RPC_HTTP_URL}" | jq -r '.result' > "$BASEDIR/nodejs/runtime_version.json"

(cd "$BASEDIR" && yarn)
(cd "$BASEDIR/nodejs" && yarn build)
(cd "$BASEDIR" && yarn lint)

# Cleanup and exit (this will be called automatically via trap)
cleanup
