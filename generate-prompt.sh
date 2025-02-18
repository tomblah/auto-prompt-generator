#!/bin/bash
set -euo pipefail

# This script is now just a thin wrapper around the new generate_prompt binary.
# It allows users to invoke generate-prompt.sh exactly as before.
# All arguments are passed directly to the Rust binary.

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Call the new generate_prompt binary, passing all the original arguments.
exec "$SCRIPT_DIR/rust/target/release/generate_prompt" "$@"
