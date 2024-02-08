#!/bin/bash

set -e

echo -- Build the Docker build image --
docker build -f Dockerfile.linux-build -t linux-build .

mkdir -p release_artifacts
docker run -t --rm --user "$(id -u)":"$(id -g)" -v "$(pwd):/workspace" -w /workspace linux-build -c 'cargo build --release'
gzip -c target/release/pact-stub-server > release_artifacts/pact-stub-server-linux-x86_64.gz
openssl dgst -sha256 -r release_artifacts/pact-stub-server-linux-x86_64.gz > release_artifacts/pact-stub-server-linux-x86_64.gz.sha256

echo -- Build the aarch64 release artifacts --
cargo install cross@0.2.5
cargo clean
cross build --target aarch64-unknown-linux-gnu --release
gzip -c target/aarch64-unknown-linux-gnu/release/pact-stub-server > release_artifacts/pact-stub-server-linux-aarch64.gz
openssl dgst -sha256 -r release_artifacts/pact-stub-server-linux-aarch64.gz > release_artifacts/pact-stub-server-linux-aarch64.gz.sha256

echo -- Build the musl release artifacts --
sudo apt install musl-tools
rustup target add x86_64-unknown-linux-musl
cargo clean
cargo build --release --target=x86_64-unknown-linux-musl
gzip -c target/x86_64-unknown-linux-musl/release/pact_mock_server_cli > release_artifacts/pact_mock_server_cli-linux-x86_64-musl.gz
openssl dgst -sha256 -r release_artifacts/pact_mock_server_cli-linux-x86_64-musl.gz > release_artifacts/pact_mock_server_cli-linux-x86_64-musl.gz.sha256

echo -- Build the musl aarch64 release artifacts --
cargo clean
cross build --release --target=aarch64-unknown-linux-musl
gzip -c target/aarch64-unknown-linux-musl/release/pact_mock_server_cli > release_artifacts/pact_mock_server_cli-linux-aarch64-musl.gz
openssl dgst -sha256 -r release_artifacts/pact_mock_server_cli-linux-aarch64-musl.gz > release_artifacts/pact_mock_server_cli-linux-aarch64-musl.gz.sha256