#!/bin/bash

set -e

echo -- Build the Docker build image --
docker build -f Dockerfile.linux-build -t linux-build .

mkdir -p target/artifacts
docker run -t --rm --user "$(id -u)":"$(id -g)" -v "$(pwd):/workspace" -w /workspace linux-build -c 'cargo build --release'
gzip -c target/release/pact-stub-server > target/artifacts/pact-stub-server-linux-x86_64.gz
openssl dgst -sha256 -r target/artifacts/pact-stub-server-linux-x86_64.gz > target/artifacts/pact-stub-server-linux-x86_64.gz.sha256

echo -- Build the aarch64 release artifacts --
cargo install cross
cross build --target aarch64-unknown-linux-gnu --release
gzip -c target/aarch64-unknown-linux-gnu/release/pact-stub-server > target/artifacts/pact-stub-server-linux-aarch64.gz
openssl dgst -sha256 -r target/artifacts/pact-stub-server-linux-aarch64.gz > target/artifacts/pact-stub-server-linux-aarch64.gz.sha256
