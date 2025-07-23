#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e
BASEDIR=$(dirname "$0")/..

set -x  # Print commands and their arguments as they are executed

"$BASEDIR/target/debug/argon-node" --chain testnet --validator \
 --sync=fast --compute-miners 0 --unsafe-force-node-key-generation \
 --rpc-port=9945 \
 --bitcoin-rpc-url=https://bitcoin:bitcoin@electrs.testnet.argonprotocol.org \
 -linfo,pallet=trace,argon=trace,txpool=trace \
 --notebook-archive-hosts=https://testnet-notebook-archive.argonprotocol.org \
