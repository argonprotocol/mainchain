#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")/..
DBPATH=postgres://postgres:password@localhost:5432/notary
# create array of validators
validators=(alice bob dave)

dropdb --if-exists -f notary;
createdb notary;
rm -rf /tmp/ulixee;
mkdir -p /tmp/ulixee/bitcoin;

# listen for sighup and kill all child processes
trap 'kill $(jobs -p)' SIGHUP SIGINT SIGTERM

# clear out iphone simulator
xcrun simctl shutdown all
xcrun simctl erase all

echo -e "(\"$BASEDIR/target/debug/ulx-testing-bitcoin\" -regtest -fallbackfee=0.0001 -listen=0 -datadir=/tmp/ulixee/bitcoin -blockfilterindex -txindex -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin &)\n\n"
$("$BASEDIR/target/debug/ulx-testing-bitcoin") -regtest -fallbackfee=0.0001 -listen=0 -datadir=/tmp/ulixee/bitcoin -blockfilterindex -txindex -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin &

# Function to check if the Bitcoin node is ready
is_node_ready() {
  curl -s --user bitcoin:bitcoin --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "getblockchaininfo", "params": [] }' -H 'content-type: text/plain;' http://127.0.0.1:18444/ | grep -q "result"
}

# Wait for the node to start
echo -e "Waiting for Bitcoin node to start...\n"
until is_node_ready; do
  sleep 1
done

# start ngrok
ngrok  --config "${BASEDIR}/scripts/ngrok.yml,$HOME/Library/Application Support/ngrok/ngrok.yml" start --all > /dev/null &

# Function to check if ngrok is ready
is_ngrok_ready() {
  curl -s http://127.0.0.1:4040/api/tunnels | grep -q "tunnels"
}

# Wait for ngrok to start
echo -e "Waiting for ngrok to start...\n\n"
until is_ngrok_ready; do
  sleep 1
done

ULX_LOCAL_TESTNET_NOTARY_URL=$(curl -s http://localhost:4040/api/tunnels/notary | jq -r '.public_url' | sed 's/https:\/\///' | sed 's/http:\/\///');

echo -e "export ULX_LOCAL_TESTNET_NOTARY_URL=\"wss://$ULX_LOCAL_TESTNET_NOTARY_URL\"\n\n"
export ULX_LOCAL_TESTNET_NOTARY_URL="wss://$ULX_LOCAL_TESTNET_NOTARY_URL"


# start a temporary node with alice and bob funded
for i in {0..0} ; do
  echo -e "(\"$BASEDIR/target/debug/ulx-node\" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --compute-miners 1 --unsafe-rpc-external --rpc-methods=unsafe --rpc-cors=all  --bitcoin-rpc-url=http://bitcoin:bitcoin@localhost:18444  &)\n\n"
  RUST_LOG=info "$BASEDIR/target/debug/ulx-node" --dev --${validators[$i]} --chain local --name=${validators[$i]}  --rpc-port=994$((i+4))  --port 3033$((i+4)) --compute-miners 1 --unsafe-rpc-external --rpc-methods=unsafe --rpc-cors=all  --bitcoin-rpc-url=http://bitcoin:bitcoin@localhost:18444 &
done

# Function to check if the Substrate node is ready
is_node_ready() {
  curl -s http://127.0.0.1:9944/health | grep -q "\"isSyncing\":false"
}

# Wait for the node to start
echo -e "Waiting for Substrate node to start...\n\n"
until is_node_ready; do
  sleep 1
done

echo -e "(\"$BASEDIR/target/debug/ulx-notary\" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary)\n\n"
"$BASEDIR/target/debug/ulx-notary" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary;

echo -e "(\"$BASEDIR/target/debug/ulx-notary\" migrate --db-url ${DBPATH})\n\n"
"$BASEDIR/target/debug/ulx-notary" migrate --db-url ${DBPATH};

echo "(\"$BASEDIR/target/debug/ulx-notary\" run --db-url ${DBPATH} -t ws://127.0.0.1:9944 --keystore-path /tmp/notary_keystore -b 0.0.0.0:9925)"
RUST_LOG=info "$BASEDIR/target/debug/ulx-notary" run --db-url ${DBPATH} -t ws://127.0.0.1:9944 --keystore-path /tmp/notary_keystore -b "0.0.0.0:9925" &

wait
