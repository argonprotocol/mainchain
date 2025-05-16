#!/bin/bash

BASEDIR=$(dirname "$0")
TARGETS_DIR=$(cd "$BASEDIR/../../target/debug" && pwd)
PASSWORD=supersecret
echo "Running targets from $TARGETS_DIR"

set -ex

MAINCHAIN_URL=ws://localhost:9944
TRUSTED_RPC_URL="$MAINCHAIN_URL"

cd $TARGETS_DIR

## if bitcoin-cli is not installed, install it
if ! command -v bitcoin-cli &> /dev/null
then
    echo "bitcoin-cli could not be found, installing..."
    brew install bitcoin
fi

# get new address from bitcoin cli
BTC_CLI="bitcoin-cli -conf=/dev/null -chain=regtest -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin"

NEW_ADDRESS=$($BTC_CLI getnewaddress addr1)
PUBKEY=$($BTC_CLI getaddressinfo $NEW_ADDRESS | jq -r '.pubkey')
BTC=${1:-0.00001}
VAULT_ID=${2:-1}
if [ -z "$1" ]
then
  printf "\n\nUsing default amount of 0.00001 BTC. Customize with first arg\n\n"
fi

./argon-bitcoin-cli lock initialize --vault-id $VAULT_ID --btc $BTC --owner-pubkey $PUBKEY
