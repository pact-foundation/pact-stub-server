#!/bin/bash
cargo clean
cargo build --release
gzip -c target/release/pact_verifier_cli.exe > target/release/pact_verifier_cli-windows-x86_64-$1.exe.gz
