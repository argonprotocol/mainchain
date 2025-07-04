#!/bin/sh
set -e

ARGS="$BITCOIN_CLI_ARGS"

# Generate blocks to a fresh address
bitcoin-cli $ARGS generatetoaddress 101 "$(bitcoin-cli $ARGS getnewaddress)"

# Wait for indexes to sync
while true; do
  INDEX_INFO=$(bitcoin-cli $ARGS getindexinfo)
  echo "$INDEX_INFO"
  TX_SYNCED=$(echo "$INDEX_INFO" | jq '.txindex.synced')
  FILTER_SYNCED=$(echo "$INDEX_INFO" | jq '."basic block filter index".synced')

  if [ "$TX_SYNCED" = "true" ] && [ "$FILTER_SYNCED" = "true" ]; then
    echo "Indexes are synced."
    break
  else
    echo "Waiting for txindex and blockfilterindex to sync..."
    sleep 2
  fi
done
