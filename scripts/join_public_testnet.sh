#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e
BASEDIR=$(dirname "$0")/..

set -x  # Print commands and their arguments as they are executed

"$BASEDIR/target/debug/argon-node" --chain testnet --validator \
 --sync=fast --compute-miners 2 --unsafe-force-node-key-generation \
        --compute-author 5DRTmdnaztvtdZ56QbEmHM8rqUR2KiKh7KY1AeMfyvkPSb5S \
 --rpc-port=9945 \
 --detailed-log-output \
 --bitcoin-rpc-url=https://bitcoin:bitcoin@electrs.testnet.argonprotocol.org \
 -linfo,pallet=trace,argon=trace,txpool=info,argon_notary_apis=info \
 --notebook-archive-hosts=https://testnet-notebook-archive.argonprotocol.org \
