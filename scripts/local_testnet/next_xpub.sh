#!/bin/bash

set -e
## if bitcoin-cli is not installed, install it
if ! command -v bitcoin-cli &> /dev/null
then
    echo "bitcoin-cli could not be found, installing..."
    brew install bitcoin
fi

# get new address from bitcoin cli
BTC_CLI="bitcoin-cli -chain=regtest -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin"

NEW_ADDRESS=$($BTC_CLI getnewaddress addr1)
PUBKEY=$($BTC_CLI getaddressinfo $NEW_ADDRESS | jq -r '.pubkey')
echo "$PUBKEY"
