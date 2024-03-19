#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")/..
DBPATH=postgres://postgres:password@localhost:5432/notary
# create array of validators
validators=(alice bob dave)

dropdb --if-exists -f notary;
createdb notary;

# listen for sighup and kill all child processes
trap 'kill $(jobs -p)' SIGHUP SIGINT SIGTERM

# start a temporary node with alice and bob funded
for i in {0..0} ; do
  echo "(\"$BASEDIR/target/release/ulx-node\" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --miners 1 &)"
  RUST_LOG=info "$BASEDIR/target/release/ulx-node" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --miners 1 &
done

echo "(\"$BASEDIR/target/release/ulx-notary\" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary)"
"$BASEDIR/target/release/ulx-notary" insert-key --keystore-path /tmp/notary_keystore --suri //Ferdie//notary;

echo "(\"$BASEDIR/target/release/ulx-node\" migrate --db-url ${DBPATH})"
"$BASEDIR/target/release/ulx-notary" migrate --db-url ${DBPATH};

echo "(\"$BASEDIR/target/release/ulx-node\" run --db-url ${DBPATH} -t ws://127.0.0.1:9944 --keystore-path /tmp/notary_keystore -b 127.0.0,1:9925)"
RUST_LOG=info "$BASEDIR/target/release/ulx-notary" run --db-url ${DBPATH} -t ws://127.0.0.1:9944 --keystore-path /tmp/notary_keystore -b "127.0.0.1:9925" &

wait
