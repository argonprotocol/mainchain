#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e
BASEDIR=$(dirname "$0")/..

set -x  # Print commands and their arguments as they are executed

RUST_LOG=info "$BASEDIR/target/release/argon-node" --chain ./node/src/chain_spec/testnet1.json --alice --compute-miners 1 --unsafe-force-node-key-generation --bitcoin-rpc-url=http://bitcoin:bitcoin@bitcoin-node.testnet.argonprotocol.org:18332
