#!/bin/bash -xe
cargo clean
cargo build --release
gzip -c target/release/pact-stub-server > target/release/pact-stub-server-osx-x86_64-$1.gz
