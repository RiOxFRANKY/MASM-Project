#!/usr/bin/env bash
# Build and run the DOSBox emulator with MAZE
set -e
cargo run --release --manifest-path dosbox/Cargo.toml -- "$@"
