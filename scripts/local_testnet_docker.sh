#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")/..
# create array of validators
validators=(alice bob dave)

# Check if minio is active on port 9000
if ! nc -z localhost 9000; then
  echo "Minio is not running on port 9000. Please start minio and try again (`./docker_minio.sh`)."
  exit 1
fi

RUNID=test01182025
DBPATH=postgres://postgres:password@localhost:5432/notary-$RUNID
# Create a minio bucket name
BUCKET_NAME="notary-$RUNID"
NOTEBOOK_ARCHIVE="http://127.0.0.1:9000/$BUCKET_NAME"

set -x  # Print commands and their arguments as they are executed
createdb notary-$RUNID || true;
mkdir -p /tmp/argon/$RUNID;
mkdir -p /tmp/argon/bitcoin;
export VERSION=1.0.1
TRAP_CMD=""

$("$BASEDIR/target/debug/argon-testing-bitcoin") -regtest -fallbackfee=0.0001 -listen=0 -datadir=/tmp/argon/bitcoin -blockfilterindex -txindex -rpcport=18444 -rpcuser=bitcoin -rpcpassword=bitcoin &

# Function to check if the Bitcoin node is ready
is_node_ready() {
  curl -s --user bitcoin:bitcoin --data-binary '{"jsonrpc": "1.0", "id": "curltest", "method": "getblockchaininfo", "params": [] }' -H 'content-type: text/plain;' http://127.0.0.1:18444/ | grep -q "result"
}

# Wait for the node to start
echo -e "Waiting for Bitcoin node to start...\n"
until is_node_ready; do
  sleep 1
done


### start ngrok
##ngrok  --config "${BASEDIR}/scripts/ngrok.yml,$HOME/Library/Application Support/ngrok/ngrok.yml" start --all > /dev/null &
##
### Function to check if ngrok is ready
##is_ngrok_ready() {
##  curl -s http://127.0.0.1:4040/api/tunnels | grep -q "tunnels"
##}
##
### Wait for ngrok to start
##echo -e "Waiting for ngrok to start...\n\n"
##until is_ngrok_ready; do
##  sleep 1
##done

argon_LOCAL_TESTNET_NOTARY_URL=$(curl -s http://localhost:4040/api/tunnels/notary | jq -r '.public_url' | sed 's/https:\/\///' | sed 's/http:\/\///');

export argon_LOCAL_TESTNET_NOTARY_URL="wss://$argon_LOCAL_TESTNET_NOTARY_URL"

# start a temporary node with alice and bob funded
for i in {0..2} ; do
  mkdir -p /tmp/argon/$RUNID/${validators[$i]};
  docker run -d --network=host --name "argon-node-${validators[$i]}" --restart=unless-stopped ghcr.io/argonprotocol/argon-miner:v$VERSION \
    --${validators[$i]} --detailed-log-output --chain local --name=${validators[$i]} \
    --base-path /tmp/argon/$RUNID/${validators[$i]} \
    --notebook-archive-hosts=$NOTEBOOK_ARCHIVE \
    --rpc-port=994$((i+4)) --port 3033$((i+4)) --compute-miners 1 \
    --pruning=archive -lRUST_LOG=info,argon=info,ismp=trace \
    --unsafe-force-node-key-generation --unsafe-rpc-external --rpc-methods=unsafe \
    --rpc-cors=all  --bitcoin-rpc-url=http://bitcoin:bitcoin@localhost:18444 &
  # remove docker on exit
  TRAP_CMD="docker rm -f argon-node-${validators[$i]} && $TRAP_CMD"
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

docker run -it --rm ghcr.io/argonprotocol/argon-notary:v$VERSION insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary;
docker run -it --rm --network=host ghcr.io/argonprotocol/argon-notary:v$VERSION migrate --db-url ${DBPATH};

# use dev to create the archive bucket for us
docker run -d --network=host -e RUST_LOG=info --name=argon-notary ghcr.io/argonprotocol/argon-notary:v$VERSION run \
  --operator-address=5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL --db-url ${DBPATH} -t ws://127.0.0.1:9944 \
  --keystore-path /tmp/notary_keystore \
  --dev \
  --archive-bucket "$BUCKET_NAME" \
  -b "0.0.0.0:9925" &

# remove docker on exit
trap "$TRAP_CMD & docker rm -f argon-notary && 'kill $(jobs -p)'" EXIT SIGHUP SIGINT SIGTERM

#echo -e "Starting a bitcoin oracle...\n\n"
#"$BASEDIR/target/debug/argon-oracle" insert-key --crypto-type=sr25519 --keystore-path /tmp/bitcoin_keystore --suri //Dave
#RUST_LOG=info "$BASEDIR/target/debug/argon-oracle" --keystore-path /tmp/bitcoin_keystore \
#  --signer-crypto=sr25519 --signer-address=5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy \
#  -t ws://127.0.0.1:9944 bitcoin --bitcoin-rpc-url=http://bitcoin:bitcoin@localhost:18444 &
#
#echo -e "Starting a pricing oracle...\n\n"
#"$BASEDIR/target/debug/argon-oracle" insert-key --crypto-type=sr25519 --keystore-path /tmp/price_keystore  --suri //Eve
#RUST_LOG=info "$BASEDIR/target/debug/argon-oracle" --keystore-path /tmp/price_keystore \
#  --signer-crypto=sr25519 --signer-address=5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw \
#  -t ws://127.0.0.1:9944 price-index --simulate-prices &
#
wait
