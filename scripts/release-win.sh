#!/bin/bash

set -e

mkdir -p target/artifacts
cargo build --release
gzip -c target/release/pact_verifier_cli.exe > target/artifacts/pact_verifier_cli-windows-x86_64.exe.gz
