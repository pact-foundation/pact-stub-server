#!/bin/bash

set -e

mkdir -p target/artifacts
cargo build --release
gzip -c target/release/pact-stub-server.exe > target/artifacts/pact-stub-server-windows-x86_64.exe.gz
