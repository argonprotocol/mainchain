#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")

subxt codegen  --derive Clone \
  --derive-for-type ulx_primitives::block_seal::BlockProof=serde::Serialize \
  --derive-for-type ulx_primitives::block_seal::SealStamper=serde::Serialize \
  --derive-for-type bounded_collections::bounded_vec::BoundedVec=serde::Serialize \
  --attributes-for-type bounded_collections::bounded_vec::BoundedVec="#[serde(transparent)]" \
   | rustfmt > "$BASEDIR/src/spec.rs"

curl -H "Content-Type: application/json" -d '{"id":"1", "jsonrpc":"2.0", "method": "state_getMetadata", "params":[]}' http://localhost:9944 > "$BASEDIR/nodejs/metadata.json"

cd "$BASEDIR/nodejs" && yarn build