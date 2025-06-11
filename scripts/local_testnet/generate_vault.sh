#!/bin/bash


BASEDIR=$(dirname "$0")
TARGETS_DIR=$(cd "$BASEDIR/../../target/debug" && pwd)
PASSWORD=supersecret
echo "Running targets from $TARGETS_DIR"

set -ex

MAINCHAIN_URL=ws://localhost:9944
TRUSTED_RPC_URL="$MAINCHAIN_URL"

cd $TARGETS_DIR

if [ ! -f /tmp/argon/xpriv_master ]; then
  ./argon-bitcoin-cli xpriv master --xpriv-path=/tmp/argon/xpriv_master --xpriv-password=$PASSWORD
fi
XPUB=$(./argon-bitcoin-cli xpriv derive-xpub --xpriv-path=/tmp/argon/xpriv_master  --xpriv-password=$PASSWORD --hd-path="m/84'/0'/0'")

./argon-bitcoin-cli vault create \
  --argons=₳50 --securitization-ratio=1x \
  --bitcoin-apr=0.5% --bitcoin-base-fee=₳0.50  \
  --liquidity-pool-profit-sharing=50% --bitcoin-xpub=$XPUB
