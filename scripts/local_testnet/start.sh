#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")/../..
# create array of validators
validators=(alice bob dave)

# Check if minio is active on port 9000
if ! nc -z localhost 9000; then
  echo "Minio is not running on port 9000. Please start minio and try again (`../docker_minio.sh`)."
  exit 1
fi

# Create a minio bucket name
RUNID=${1:-$(date "+%Y%m%d-%H%M%S")}
BUCKET_NAME="notary-$RUNID"
NOTEBOOK_ARCHIVE="http://127.0.0.1:9000/$BUCKET_NAME"
DBPATH=postgres://postgres:password@localhost:5432/notary-$RUNID

set -x  # Print commands and their arguments as they are executed
# drop db if no arg is passed
if [ -z "$1" ]; then
  dropdb --if-exists -f notary-$RUNID;
  rm -rf /tmp/argon/$RUNID;
fi
createdb notary-$RUNID || true;
mkdir -p /tmp/argon/$RUNID;
mkdir -p /tmp/argon/$RUNID/bitcoin;

# listen for sighup and kill all child processes
trap 'kill $(jobs -p)' SIGHUP SIGINT SIGTERM

$("$BASEDIR/target/debug/argon-testing-bitcoin") -conf=/dev/null -chain=regtest -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin \
  -fallbackfee=0.0001 -listen=0 -datadir=/tmp/argon/$RUNID/bitcoin \
  -blockfilterindex -txindex -wallet=1  2>&1 | \
  awk -v name="bitcoin" '{printf "%-8s %s\n", name, $0; fflush()}' &

if ! command -v bitcoin-cli &> /dev/null
then
    echo "bitcoin-cli could not be found, installing..."
    brew install bitcoin
fi
# Function to check if the Bitcoin node is ready
is_node_ready() {
  curl -s --user bitcoin:bitcoin --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "getblockchaininfo", "params": [] }' -H 'content-type: text/plain;' http://127.0.0.1:18444/ | grep -q "result"
}

# Wait for the node to start
echo -e "Waiting for Bitcoin node to start...\n"
until is_node_ready; do
  sleep 1
done


BTC_CLI="bitcoin-cli -conf=/dev/null -chain=regtest -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin"
# try to create wallet, but if it already exists, ignore error
$BTC_CLI createwallet "default" || $BTC_CLI loadwallet "default" || true

# go to point of generating funds
$BTC_CLI -generate 101


# If we're using iphone simulator (ACTIVATE_IOS), set up ngrok and delete all simulators
if [ "$ACTIVATE_IOS" = "true" ] && [ -n "$ACTIVATE_IOS" ]; then
  # clear out iphone simulator
  xcrun simctl shutdown all
  xcrun simctl erase all

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
  ARGON_LOCAL_TESTNET_NOTARY_URL=$(curl -s http://localhost:4040/api/tunnels/notary | jq -r '.public_url' | sed 's/https:\/\///' | sed 's/http:\/\///');

  export ARGON_LOCAL_TESTNET_NOTARY_URL="wss://$ARGON_LOCAL_TESTNET_NOTARY_URL"
fi

# start a temporary node with alice and bob funded
for i in {0..2} ; do
  mkdir -p /tmp/argon/$RUNID/${validators[$i]};
  "$BASEDIR/target/debug/argon-node" \
    --"${validators[$i]}" --detailed-log-output --chain local --name="${validators[$i]}" \
    --bootnodes=/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWMdmKGEuFPVvwSd92jCQJgX9aFCp45E8vV2X284HQjwnn \
    --notebook-archive-hosts="$NOTEBOOK_ARCHIVE" \
    --base-path /tmp/argon/$RUNID/${validators[$i]} \
    --rpc-port=994$((i+4)) --port 3033$((i+4)) --compute-miners 1 \
    --pruning=archive -lRUST_LOG=info,argon=trace,pallet=trace,argon_notary_apis=info,argon_node_consensus::notary_client=info,argon_node_consensus::aux_client=info \
    --unsafe-rpc-external --rpc-methods=unsafe \
    --no-mdns \
    $(if [ "$i" -eq 0 ]; then echo "--node-key=16ec4f460237d066d15d09a44959a7d49ea6405e98429826f1c28b9087bd60ea"; else echo "--unsafe-force-node-key-generation"; fi) \
    --rpc-cors=all  --bitcoin-rpc-url=http://bitcoin:bitcoin@localhost:18444 2>&1 | \
    awk -v name="${validators[$i]}" '{printf "%-8s %s\n", name, $0; fflush()}' &
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

"$BASEDIR/target/debug/argon-notary" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary;

"$BASEDIR/target/debug/argon-notary" migrate --db-url ${DBPATH};

# use dev to create the archive bucket for us
RUST_LOG=info "$BASEDIR/target/debug/argon-notary" run \
  --operator-address=5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL --db-url ${DBPATH} -t ws://127.0.0.1:9944 \
  --keystore-path /tmp/notary_keystore \
  --archive-bucket "$BUCKET_NAME" \
  --dev \
  -b "0.0.0.0:9925" 2>&1 | \
  awk -v name="notary" '{printf "%-8s %s\n", name, $0; fflush()}' &

echo -e "Starting a bitcoin oracle...\n\n"
"$BASEDIR/target/debug/argon-oracle" insert-key --crypto-type=sr25519 --keystore-path /tmp/bitcoin_keystore --suri //Dave
RUST_LOG=info "$BASEDIR/target/debug/argon-oracle" --keystore-path /tmp/bitcoin_keystore \
  --signer-crypto=sr25519 --signer-address=5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy \
  -t ws://127.0.0.1:9944 bitcoin --bitcoin-rpc-url=http://bitcoin:bitcoin@localhost:18444  2>&1 | \
  awk -v name="orclbtc" '{printf "%-8s %s\n", name, $0; fflush()}' &

echo -e "Starting a pricing oracle...\n\n"
"$BASEDIR/target/debug/argon-oracle" insert-key --crypto-type=sr25519 --keystore-path /tmp/price_keystore  --suri //Eve
ORACLE_CPI_CACHE_PATH=/tmp/oracle/data/US_CPI_State.json RUST_LOG=info "$BASEDIR/target/debug/argon-oracle" --keystore-path /tmp/price_keystore \
  --signer-crypto=sr25519 --signer-address=5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw \
  -t ws://127.0.0.1:9944 price-index --simulate-prices   2>&1 | \
  awk -v name="orclprc" '{printf "%-8s %s\n", name, $0; fflush()}' &

wait
