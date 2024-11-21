#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -eux

# move to this directory as the working directory
cd "$(dirname "$0")"

docker run -v ./paseo-relayer.toml:/root/consensus.toml --platform linux/amd64 -e RUST_LOG=info,jsonrpsee=trace polytopelabs/tesseract-consensus:latest \
  --config=/root/consensus.toml log-consensus-state KUSAMA-4009
