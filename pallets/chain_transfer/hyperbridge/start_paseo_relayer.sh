#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -eux

# move to this directory as the working directory
cd "$(dirname "$0")"

docker run -v ./paseo-relayer.toml:/root/consensus.toml \
  --name=paseo-hyperbridge-consensus \
  --platform linux/amd64 \
  -e RUST_LOG=info \
  polytopelabs/tesseract-consensus:latest \
  --config=/root/consensus.toml &

docker run -v ./paseo-messaging-relayer.toml:/home/root/config.toml \
  --platform linux/amd64 \
  -e RUST_LOG=info \
  --name=paseo-hyperbridge-messaging \
  --network=host \
  --restart=always \
  polytopelabs/tesseract:latest \
  --config=/home/root/config.toml \
  --db=/home/root/tesseract.db &
