#!/bin/bash
set -euo pipefail

# Run rust tests
cargo test --manifest-path rust/filter_files_singular/Cargo.toml

# Check if bats is installed.
if ! command -v bats >/dev/null 2>&1; then
    echo "Error: bats is not installed. Please install bats from https://github.com/bats-core/bats-core."
    exit 1
fi

# Run all BATS test files (*.bats) in the current directory.
bats *.bats
