#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

BASEDIR=$(dirname "$0")/..

# create array of validators
validators=(alice bob dave)

# listen for sighup and kill all child processes
trap 'kill $(jobs -p)' SIGHUP SIGINT SIGTERM

# start a temporary node with alice and bob funded
for i in {0..2} ; do
  echo "(\"$BASEDIR/target/release/ulx-node\" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --miners 1 &)"
  "$BASEDIR/target/release/ulx-node" --tmp --${validators[$i]} --chain local --rpc-port=994$((i+4))  --port 3033$((i+4)) --miners 1 &
done

wait
