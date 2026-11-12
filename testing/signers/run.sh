#!/usr/bin/env bash
set -euo pipefail

# bash testing/signers/run.sh [vault|ledger|yubikey|all]
# Ledger requires Docker and authenticated gh. YubiKey requires a physical PIV device.
# Downloads/builds stay outside Git. Set ARGON_SIGNER_CACHE to reuse them across runs.
repo_dir="$(git rev-parse --show-toplevel)"
signer_dir="$repo_dir/testing/signers"
signer_cache="${ARGON_SIGNER_CACHE:-$(mktemp -d "${TMPDIR:-/tmp}/argon-signer-tests.XXXXXX")}"
selection="${1:-all}"
case "$selection" in
  vault|ledger|yubikey|all) ;;
  *) echo "Expected vault, ledger, yubikey or all" >&2; exit 2 ;;
esac
mkdir -p "$signer_cache"
export ARGON_SIGNER_CACHE="$signer_cache"

if [[ "$selection" == yubikey ]]; then
  : "${ARGON_YUBIKEY_SLOT:?Set ARGON_YUBIKEY_SLOT to an existing ECCP256 signing slot with a certificate}"
  for tool in yubico-piv-tool openssl; do
    if ! command -v "$tool" >/dev/null; then
      echo "Missing required command: $tool" >&2
      exit 1
    fi
  done
  export ARGON_YUBIKEY_TEST_DIR="$(mktemp -d "$signer_cache/yubikey.XXXXXX")"
  cd "$repo_dir"
  CARGO_BUILD_BUILD_DIR="${CARGO_BUILD_BUILD_DIR:-$signer_cache/runtime-build}" \
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$signer_cache/runtime-target}" \
    CARGO_INCREMENTAL=0 SKIP_WASM_BUILD=1 cargo test -p argon-runtime --lib \
    tests::account_security::yubikey_piv_enrolls_and_authorizes_without_a_funded_signer \
    -- --exact --ignored --nocapture
  exit
fi

ledger_container=""
cleanup() {
  if [[ -n "$ledger_container" ]]; then
    "${docker_cli[@]}" stop "$ledger_container" >/dev/null
  fi
}
trap cleanup EXIT

if [[ "$selection" != vault ]]; then
  runtime_target="${CARGO_TARGET_DIR:-$signer_cache/runtime-target}"
  runtime_build="${CARGO_BUILD_BUILD_DIR:-$signer_cache/runtime-build}"
  cd "$repo_dir"
  CARGO_BUILD_BUILD_DIR="$runtime_build" CARGO_TARGET_DIR="$runtime_target" \
    CARGO_INCREMENTAL=0 SKIP_WASM_BUILD=1 cargo build -p argon-runtime --lib
  rustc --edition=2021 "$signer_dir/export_metadata.rs" \
    -L "dependency=$runtime_build/debug/deps" \
    --extern "argon_runtime=$runtime_target/debug/libargon_runtime.rlib" \
    -o "$signer_cache/export-metadata"
  "$signer_cache/export-metadata" "$signer_cache/runtime-v15.scale"
  export ARGON_SIGNER_METADATA_V15="$signer_cache/runtime-v15.scale"

  ledger_source="$signer_cache/ledger-polkadot"
  if [[ ! -d "$ledger_source/.git" ]]; then
    gh repo clone Zondax/ledger-polkadot "$ledger_source" -- \
      --depth 1 --branch v100.0.30 --recurse-submodules --shallow-submodules
  fi
  if [[ "$(git -C "$ledger_source" rev-parse HEAD)" != 7520e0e5ee822ad8c04f5003025124a146a1612e ]]; then
    echo "Ledger source does not match the pinned v100.0.30 commit" >&2
    exit 1
  fi

  # Public images need no registry credentials. Do not modify the user's Docker config.
  docker_host="${DOCKER_HOST:-$(docker context inspect --format '{{.Endpoints.docker.Host}}')}"
  mkdir -p "$signer_cache/docker"
  docker_cli=(docker --config "$signer_cache/docker" --host "$docker_host")
  builder="zondax/ledger-app-builder@sha256:207c58f7de1921f4e7cf1a23c2173b1cc0820aa8529ce83bbe04199968ef5240"
  speculos="ghcr.io/ledgerhq/speculos@sha256:56c7ec84c903147dac28e68706d3241218a3d9beb9562dd4a8c02947b2a24f47"
  "${docker_cli[@]}" pull "$builder"
  "${docker_cli[@]}" pull "$speculos"
  "${docker_cli[@]}" run --rm -e SDK_VARNAME=NANOSP_SDK -e TARGET=nanos2 \
    -e APP_TESTING=1 -e PRODUCTION_BUILD=1 -v "$ledger_source:/app" "$builder" 'make -j 4'
  ledger_container="$("${docker_cli[@]}" run --detach --rm \
    -p 127.0.0.1::5000 -p 127.0.0.1::9999 \
    -v "$ledger_source/app/build/nanos2/bin/app.elf:/app.elf:ro" \
    "$speculos" /app.elf --model nanosp --display headless --api-port 5000 --apdu-port 9999)"
  rest_address="$("${docker_cli[@]}" port "$ledger_container" 5000/tcp)"
  apdu_address="$("${docker_cli[@]}" port "$ledger_container" 9999/tcp)"
  export ARGON_SPECULOS_URL="http://$rest_address"
  export ARGON_SPECULOS_APDU_PORT="${apdu_address##*:}"
fi

cd "$signer_dir"
case "$selection" in
  all) yarn test ;;
  *) yarn test "$selection.e2e.test.ts" ;;
esac
