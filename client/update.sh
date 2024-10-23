#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")
PIPE="/tmp/argon-node-output"
rm -f "$PIPE"
mkfifo "$PIPE"

# Function to clean up before exit
cleanup() {
    echo "Cleaning up..."
    rm -f "$PIPE"
    kill $argon_PID
    wait $argon_PID 2>/dev/null
}

trap cleanup EXIT SIGHUP SIGINT SIGTERM

"$BASEDIR/../target/debug/argon-node" --tmp --dev --alice --rpc-port=9944 --compute-miners=1 --bitcoin-rpc-url="http://127.0.0.1:18443" > "$PIPE" 2>&1 &
argon_PID=$!

while IFS= read -r line; do
    echo "$line"
    if [[ "$line" == *"Running JSON-RPC server: addr=127.0.0.1:9944"* ]]; then
        echo "Detected JSON-RPC server startup."
        break
    fi
done <"$PIPE"

subxt codegen  --derive Clone \
  --derive-for-type bounded_collections::bounded_vec::BoundedVec=serde::Serialize \
  --attributes-for-type bounded_collections::bounded_vec::BoundedVec="#[serde(transparent)]" \
  --substitute-type primitive_types::H256=crate::types::H256 \
  --substitute-type sp_core::crypto::AccountId32=crate::types::AccountId32 \
   | rustfmt > "$BASEDIR/src/spec.rs"

curl -H "Content-Type: application/json" -d '{"id":"1", "jsonrpc":"2.0", "method": "state_getMetadata", "params":[]}' http://localhost:9944 > "$BASEDIR/nodejs/metadata.json"

cd "$BASEDIR" && yarn
cd "nodejs" && yarn build

# Cleanup and exit (this will be called automatically via trap)
cleanup
