#!/usr/bin/env bash
set -eEuvx

source $HOME/.cargo/env

# XCode tries to be helpful and overwrites the PATH. Reset that.
PATH="$(bash -l -c 'echo $PATH')"

INPUT_FILE_PATH=${SCRIPT_INPUT_FILE_0}

$HOME/.cargo/bin/cargo run -p uniffi-bindgen-cli -- generate --library "${INPUT_FILE_PATH}" --language swift --out-dir "./Generated"

