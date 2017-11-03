#!/bin/bash -xe
cargo clean
cargo build --release --target x86_64-apple-ios
gzip -c target/x86_64-apple-ios/release/pact-stub-server > target/x86_64-apple-ios/release/pact-stub-server-ios-x86_64-$1.gz
