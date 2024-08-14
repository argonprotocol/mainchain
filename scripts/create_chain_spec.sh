#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e
BASEDIR=$(dirname "$0")/..

./target/release/argon-node build-spec --chain testnet > $BASEDIR/specs/testnet.json
./target/release/argon-node build-spec --chain testnet --raw > $BASEDIR/specs/testnet-raw.json
