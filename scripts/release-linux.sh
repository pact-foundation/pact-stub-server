#!/bin/bash

set -e

echo -- Build the Docker build image --
docker build -f Dockerfile.linux-build -t linux-build .

mkdir -p target/artifacts
docker run -t --rm --user "$(id -u)":"$(id -g)" -v "$(pwd):/workspace" -w /workspace linux-build -c 'cargo build --release'
gzip -c target/release/pact-stub-server > target/artifacts/pact-stub-server-linux-x86_64.gz
