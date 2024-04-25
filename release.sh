#!/bin/bash

set -e
set -x

RUST_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd )"

source "$RUST_DIR/scripts/gzip-and-sum.sh"
ARTIFACTS_DIR=${ARTIFACTS_DIR:-"$RUST_DIR/release_artifacts"}
mkdir -p "$ARTIFACTS_DIR"
export CARGO_TARGET_DIR=${CARO_TARGET_DIR:-"$RUST_DIR/target"}

if [ $# -lt 1 ]
then
    echo "Usage : $0 <Linux|Windows|macOS> <cargo flags>"
    exit
fi

APP=pact-stub-server
OS=$1
shift;
echo Building Release for "$OS"
# All flags passed to this script are passed to cargo.
cargo_flags=( "$@" )
install_cross() {
    cargo install cross@0.2.5
}

build_linux_x86_64() {
    install_cross
    cargo clean
    cross build --target=x86_64-unknown-linux-musl "${cargo_flags[@]}"
    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/${APP}" \
            "$ARTIFACTS_DIR/${APP}-linux-x86_64.gz"

    fi
}

build_linux_aarch64() {
    install_cross
    cargo clean
    cross build --target=aarch64-unknown-linux-musl "${cargo_flags[@]}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-unknown-linux-musl/release/${APP}" \
            "$ARTIFACTS_DIR/${APP}-linux-aarch64.gz"
    fi
    #cargo clean
}
# Build the x86_64 darwin release
build_macos_x86_64() {
    cargo build --target x86_64-apple-darwin "${cargo_flags[@]}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-apple-darwin/release/${APP}" \
            "$ARTIFACTS_DIR/${APP}-osx-x86_64.gz"
        gzip_and_sum \
                    "$CARGO_TARGET_DIR/x86_64-apple-darwin/release/${APP}" \
                    "$ARTIFACTS_DIR/${APP}-macos-x86_64.gz"
    fi
}

# Build the aarch64 darwin release
build_macos_aarch64() {
    cargo build --target aarch64-apple-darwin "${cargo_flags[@]}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/${APP}" \
            "$ARTIFACTS_DIR/${APP}-osx-aarch64.gz"
        gzip_and_sum \
                    "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/${APP}" \
                    "$ARTIFACTS_DIR/${APP}-macos-aarch64.gz"
    fi
}

# Build the x86_64 windows release
build_windows_x86_64() {
    cargo build --target x86_64-pc-windows-msvc "${cargo_flags[@]}"

    # If --release in cargo flags, then gzip and sum the release artifacts
    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-pc-windows-msvc/release/${APP}.exe" \
            "$ARTIFACTS_DIR/${APP}-windows-x86_64.exe.gz"
    fi
}

# Build the aarch64 windows release
build_windows_aarch64() {
    cargo build --target aarch64-pc-windows-msvc "${cargo_flags[@]}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-pc-windows-msvc/release/${APP}.exe" \
            "$ARTIFACTS_DIR/${APP}-windows-aarch64.exe.gz"
    fi
}

case "$OS" in
  Linux)    echo "Building for Linux"
            build_linux_x86_64
            build_linux_aarch64
            ;;
  Windows)  echo "Building for windows"
            build_windows_x86_64
            build_windows_aarch64
            ;;
  macOS)    echo  "Building for macos"
            build_macos_x86_64
            build_macos_aarch64
            ;;
  *)        echo "$OS is not a recognised OS"
            exit 1
            ;;
esac