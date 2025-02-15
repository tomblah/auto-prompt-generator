#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects the contents of various files in the repository and copies them to the clipboard.
#
# It includes:
# - All .sh and README* files
# - Optionally .bats files (if --include-tests or --tests-only is passed)
# - Optionally only Rust source files (if --rust-only is passed)
#
# Usage:
#   ./meta-context.sh [--include-tests] [--tests-only] [--rust-only]
#
# Options:
#   --include-tests  : Includes .bats test files along with .sh and README* files.
#   --tests-only     : Includes only .bats files.
#   --rust-only      : Includes only Rust source files (.rs) under the rust directory.
#
##########################################

# Parse command-line options
INCLUDE_TESTS=false
TESTS_ONLY=false
RUST_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --include-tests)
            INCLUDE_TESTS=true
            shift
            ;;
        --tests-only)
            TESTS_ONLY=true
            shift
            ;;
        --rust-only)
            RUST_ONLY=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Ensure mutually exclusive options are not used together.
if { $INCLUDE_TESTS || $TESTS_ONLY; } && $RUST_ONLY; then
  echo "Error: Cannot use --rust-only with --include-tests or --tests-only." >&2
  exit 1
fi

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Optionally, determine the repository root (assumes you are in a Git repository).
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

# Build the find command based on the options provided.
if $RUST_ONLY; then
    echo "Including only Rust source files in the context."
    files=$(find rust -type f -iname "*.rs")
elif $TESTS_ONLY; then
    echo "Including only .bats test files in the context."
    files=$(find . -type f -iname "*.bats" \
            -not -name "meta-context.sh" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
elif $INCLUDE_TESTS; then
    echo "Including .bats files along with .sh and README* files in the context."
    files=$(find . -type f \( -iname "*.sh" -o -iname "README*" -o -iname "*.bats" \) \
            -not -name "meta-context.sh" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
else
    files=$(find . -type f \( -iname "*.sh" -o -iname "README*" \) \
            -not -name "meta-context.sh" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
    
    # Additionally, include Rust source files if the rust directory exists.
    if [ -d "rust" ]; then
        echo "Including Rust source files from rust in the context."
        rust_files=$(find rust -type f -iname "*.rs")
        files="$files $rust_files"
    fi
fi

# --------------------------------------------------
# Include rust/Cargo.toml if it exists.
if [ -f "rust/Cargo.toml" ]; then
    echo "Including rust/Cargo.toml in the context."
    files="$files rust/Cargo.toml"
fi
# --------------------------------------------------

# Display the collected files.
echo "--------------------------------------------------"
echo "Files to include in the meta-context prompt:"
for file in $files; do
    echo " - $file"
done
echo "--------------------------------------------------"

# Create a temporary file to accumulate the context.
temp_context=$(mktemp)

# Loop over each file and append a header and its content.
for file in $files; do
    {
      echo "--------------------------------------------------"
      echo "The contents of $file is as follows:"
      echo "--------------------------------------------------"
      cat "$file"
      echo -e "\n"
    } >> "$temp_context"
done

# Append the final custom message based on the option provided.
if $TESTS_ONLY; then
    {
      echo "--------------------------------------------------"
      echo -e "Can you look through these tests and add unit tests to cover the functionality we've added.\n\nLet's lean towards appending to existing files where it makes sense to do so. However, if we've created a new script, it might make sense to create a new test file for it. And be sure to echo out the entire test file with the added test cases."
    } >> "$temp_context"
elif ! $RUST_ONLY; then
    {
      echo "--------------------------------------------------"
      echo -e "I'm improving the generate-prompt.sh functionality (see README above for more context). I'm trying to keep generate-prompt.sh as thin as possible, so try not to propose solutions that edit it unless where it makes obvious sense to, e.g. for parsing options. But if there is an easy solution to create another file, or edit another existing file, let's prefer that. For any new files we create, let's do it rust, not bash.\n\n"
    } >> "$temp_context"
fi

# Copy the final context to the clipboard using pbcopy (macOS).
# For Linux, you might use: xclip -selection clipboard or xsel --clipboard --input.
cat "$temp_context" | pbcopy

echo "--------------------------------------------------"
echo "Success: Meta context has been copied to the clipboard."
echo "--------------------------------------------------"

# Clean up the temporary file.
rm "$temp_context"
