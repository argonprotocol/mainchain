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

# listen for sighup and kill all child processes
trap 'kill $(jobs -p)' SIGHUP SIGINT SIGTERM

xcrun simctl shutdown all
xcrun simctl erase all

# start ngrok
ngrok  --config "${BASEDIR}/scripts/ngrok.yml,$HOME/Library/Application Support/ngrok/ngrok.yml" start --all > /dev/null &
sleep 1

ULX_LOCAL_TESTNET_NOTARY_URL=$(curl -s http://localhost:4040/api/tunnels/notary | jq -r '.public_url' | sed 's/https:\/\///' | sed 's/http:\/\///');

echo "export ULX_LOCAL_TESTNET_NOTARY_URL=\"wss://$ULX_LOCAL_TESTNET_NOTARY_URL\""
export ULX_LOCAL_TESTNET_NOTARY_URL="wss://$ULX_LOCAL_TESTNET_NOTARY_URL"


# start a temporary node with alice and bob funded
for i in {0..0} ; do
  echo "(\"$BASEDIR/target/debug/ulx-node\" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --miners 1 &)"
  RUST_LOG=info "$BASEDIR/target/debug/ulx-node" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --miners 1 --unsafe-rpc-external --rpc-methods=unsafe --rpc-cors=all &
done

echo "(\"$BASEDIR/target/debug/ulx-notary\" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary)"
"$BASEDIR/target/debug/ulx-notary" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary;

echo "(\"$BASEDIR/target/debug/ulx-node\" migrate --db-url ${DBPATH})"
"$BASEDIR/target/debug/ulx-notary" migrate --db-url ${DBPATH};

echo "(\"$BASEDIR/target/debug/ulx-node\" run --db-url ${DBPATH} -t ws://127.0.0.1:9944 --keystore-path /tmp/notary_keystore -b 0.0.0.0:9925)"
RUST_LOG=info "$BASEDIR/target/debug/ulx-notary" run --db-url ${DBPATH} -t ws://127.0.0.1:9944 --keystore-path /tmp/notary_keystore -b "0.0.0.0:9925" &

wait
