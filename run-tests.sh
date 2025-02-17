#!/bin/bash
set -euo pipefail

# Run Rust tests for all packages in the rust directory.
# This loop finds every Cargo.toml under "rust" and runs its tests.
while IFS= read -r -d '' manifest; do
    package_dir=$(dirname "$manifest")
    echo "Running tests in package: $package_dir"
    cargo test --manifest-path "$manifest" -- --test-threads=1
done < <(find rust -name Cargo.toml -print0)

# Check if bats is installed.
if ! command -v bats >/dev/null 2>&1; then
    echo "Error: bats is not installed. Please install bats from https://github.com/bats-core/bats-core."
    exit 1
fi

# Run all BATS test files (*.bats) in the current directory.
bats *.bats
