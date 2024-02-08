#!/bin/bash

set -e

mkdir -p release_artifacts
cargo build --release
gzip -c target/release/pact-stub-server.exe > release_artifacts/pact-stub-server-windows-x86_64.exe.gz
