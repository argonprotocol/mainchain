#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e
BASEDIR=$(dirname "$0")/..

set -x  # Print commands and their arguments as they are executed

echo "You must first export the state from an existing testnet node, e.g.:"
echo "ssh root@testnet.miner1

   systemctl stop argon-testnet

  /home/argon/bin/argon-testnet/app export-state \
   --chain testnet --bitcoin-rpc-url=http://bitcoin:bitcoin@electrs.testnet.argonprotocol.org:38332 \
   --base-path /home/argon/.local/share/argon-node \
   --log=error > state.raw
"
echo "Once you have the state.raw file, copy it to this machine and run this script again.

  scp root@testnet.miner1:~/state.raw ."

read -p "Press enter once completed..."

if [ ! -f state.raw ]; then
  echo "state.raw file not found! Exiting."
  exit 1
fi

mkdir -p /tmp/overrides
cp "$BASEDIR"/target/debug/wbuild/argon-canary-runtime/argon_canary_runtime.wasm /tmp/overrides/

"$BASEDIR/target/debug/argon-node" \
  --chain state.raw \
  --base-path /tmp/testnet \
  --wasm-runtime-overrides /tmp/overrides \
  --rpc-methods Unsafe \
  --validator \
  --no-telemetry \
  --no-mdns \
  --reserved-only \
  --in-peers 0 \
  --out-peers 0 \
  --rpc-cors all \
  --rpc-port 9944 \
  --execution=wasm \
  --wasm-execution=compiled \
  --compute-miners 2 \
--unsafe-force-node-key-generation \
  --compute-author 5DRTmdnaztvtdZ56QbEmHM8rqUR2KiKh7KY1AeMfyvkPSb5S \
 --detailed-log-output \
 --bitcoin-rpc-url=https://bitcoin:bitcoin@electrs.testnet.argonprotocol.org \
 -linfo,pallet=trace,argon=trace,txpool=info,argon_notary_apis=info \
 --notebook-archive-hosts=https://testnet-notebook-archive.argonprotocol.org \
  --no-prometheus
