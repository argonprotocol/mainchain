#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

#cargo build --features=try-runtime --release

# disable multiblock checks since it cant create blocks
# Replace the prefix with the module from the storage for the pallet(s) you want to download
try-runtime \
    --runtime ./target/release/wbuild/argon-canary-runtime/argon_canary_runtime.compact.compressed.wasm \
    on-runtime-upgrade \
    --print-storage-diff \
    --blocktime 60000 \
    --disable-mbm-checks \
    live \
      --uri wss://rpc.testnet.argonprotocol.org \
      --prefix 8200ee1e7ffeceaefd524b3ceb4c2a19 \
