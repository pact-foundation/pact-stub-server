#!/bin/bash -xe

set -e

mkdir -p target/artifacts
cargo build --release
gzip -c target/release/pact-stub-server > target/artifacts/pact-stub-server-linux-x86_64.gz
