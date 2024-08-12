#!/usr/bin/env bash
set -eEuvx

source $HOME/.cargo/env
function error_help()
{
    ERROR_MSG="It looks like something went wrong building the Rust Localchain Binary."
    echo "error: ${ERROR_MSG}"
}
trap error_help ERR

# XCode tries to be helpful and overwrites the PATH. Reset that.
PATH="$(bash -l -c 'echo $PATH')"

# This should be invoked from inside xcode, not manually
if [[ "${#}" -ne 2 ]]
then
    echo "Usage (note: only call inside xcode!):"
    echo "path/to/build-scripts/build-binary.sh <SRC_ROOT_PATH> <buildvariant>"
    exit 1
fi

# path to source code root
SRC_ROOT=${1}
# buildvariant from our xcconfigs
BUILDVARIANT=$(echo "${2}" | tr '[:upper:]' '[:lower:]')

FEATURES=uniffi
RELFLAG=
if [[ "${BUILDVARIANT}" != "debug" ]]; then
    RELFLAG=--release
    FEATURES=uniffi
fi

#if [[ -n "${SDK_DIR:-}" ]]; then
#  # Assume we're in Xcode, which means we're probably cross-compiling.
#  # In this case, we need to add an extra library search path for build scripts and proc-macros,
#  # which run on the host instead of the target.
#  # (macOS Big Sur does not have linkable libraries in /usr/lib/.)
#  export LIBRARY_PATH="${SDK_DIR}/usr/lib:${LIBRARY_PATH:-}"
#fi

cd "${SRC_ROOT}/localchain"

IS_SIMULATOR=0
if [ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]; then
  IS_SIMULATOR=1
fi

OS_TARGET=""
for arch in $ARCHS; do
  case "$arch" in
    x86_64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        echo "Building for x86_64, but not a simulator build. What's going on?" >&2
        exit 2
      fi

      # Intel iOS simulator
      export CFLAGS_x86_64_apple_ios="-target x86_64-apple-ios"
      OS_TARGET="x86_64-apple-ios"
      ;;

    arm64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        # Hardware iOS targets
        OS_TARGET="aarch64-apple-ios"
      else
        # M1 iOS simulator
        OS_TARGET="aarch64-apple-ios-sim"
      fi
  esac

  export RUSTC_WRAPPER=sccache
  $HOME/.cargo/bin/cargo build -p argon-localchain --features=$FEATURES --lib $RELFLAG --target "${OS_TARGET}"
done

cp "../target/${OS_TARGET}/${BUILDVARIANT}/libargon_localchain.a" "./apple/LocalchainIOS"
cp "../target/${OS_TARGET}/${BUILDVARIANT}/libargon_localchain.d" "./apple/LocalchainIOS"
