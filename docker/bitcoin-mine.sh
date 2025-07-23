#!/bin/sh
set -e

ARGS="$BITCOIN_CLI_ARGS"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-10}"
BLOCKS_PER_ROUND="${BLOCKS_PER_ROUND:-1}"

echo "Starting miner loop: every $INTERVAL_SECONDS seconds, generate $BLOCKS_PER_ROUND blocks."

while true; do
  ADDR=$(bitcoin-cli $ARGS getnewaddress)
  echo "Mining $BLOCKS_PER_ROUND block(s) to $ADDR"
  bitcoin-cli $ARGS generatetoaddress $BLOCKS_PER_ROUND "$ADDR"
  sleep "$INTERVAL_SECONDS"
done
