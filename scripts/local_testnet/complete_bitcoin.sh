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

# try to create wallet, but if it already exists, ignore error
$BTC_CLI createwallet "default" || $BTC_CLI loadwallet "default" || true

# utxo id should be the first arg, or print a message
if [ -z "$1" ]
then
  echo "Please provide utxo id as first argument"
  exit 1
fi
UTXO_ID=$1

RESP=$(./argon-bitcoin-cli lock send-to-address --utxo-id $UTXO_ID)

# Using capture groups, parse the address and sats out of "You must send exactly 1000 satoshis to bcrt1q8s3de3j38cx0hd5l9z33mrlkp3rszdhczme2q4zrzwws3uxpejds8f6cl7, which is a multisig with your public key 02ea26e0b6a6697495333d52c7ff2504642f59d6eb158ddf7b424f620f90d20ed7."
ADDRESS=$(echo $RESP | grep -o "satoshis to [0-9a-zA-Z]\+" | grep -o "[0-9a-zA-Z]\+" | tail -n 1)
SATS=$(echo $RESP | grep -o "exactly [0-9]\+ satoshis" | grep -o "[0-9]\+")
BTC=$(echo $SATS | awk '{print $1 / 100000000}')

# send the sats to the address
$BTC_CLI sendtoaddress $ADDRESS $BTC

$BTC_CLI -generate 7
