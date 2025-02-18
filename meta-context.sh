#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects the contents of various files in the repository and copies them to the clipboard.
#
# Options:
#   --include-scripts : In addition to Rust source files, also include .sh and README* files.
#   --include-tests   : When including scripts, also include .bats test files.
#   --tests-only      : Include only .bats test files.
##########################################

# Define a function to filter out inline Rust test blocks.
filter_rust_tests() {
    awk '
    BEGIN { in_tests=0; brace_count=0 }
    {
        # If we see a #[cfg(test)] attribute and we are not already in a test block, start skipping.
        if (in_tests == 0 && $0 ~ /^[[:space:]]*#\[cfg\(test\)\]/) {
            in_tests = 1;
            next;
        }
        # If we are in a test block and see a module declaration, start counting braces.
        if (in_tests == 1 && $0 ~ /^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{/) {
            brace_count = 1;
            next;
        }
        # If we are inside a test module block, count braces to know when it ends.
        if (in_tests == 1 && brace_count > 0) {
            n = gsub(/\{/, "{");
            m = gsub(/\}/, "}");
            brace_count += n - m;
            if (brace_count <= 0) {
                in_tests = 0;
                brace_count = 0;
            }
            next;
        }
        print;
    }
    ' "$1"
}

# Parse command-line options.
INCLUDE_SCRIPTS=false
INCLUDE_TESTS=false
TESTS_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --include-scripts)
            INCLUDE_SCRIPTS=true
            shift
            ;;
        --include-tests)
            INCLUDE_TESTS=true
            shift
            ;;
        --tests-only)
            TESTS_ONLY=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Prevent combining mutually exclusive options.
if $TESTS_ONLY && $INCLUDE_SCRIPTS; then
  echo "Error: Cannot use --tests-only with --include-scripts." >&2
  exit 1
fi

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Optionally, determine the repository root (assumes you're in a Git repository).
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

# Build the file list based on the options provided.
if $TESTS_ONLY; then
    echo "Including only .bats test files in the context."
    files=$(find . -type f -iname "*.bats" \
            -not -name "meta-context.sh" \
            -not -path "./.git/*" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
elif $INCLUDE_SCRIPTS; then
    echo "Including bash scripts, README*, and Rust source files in the context."
    files=$(find . -type f \( -iname "*.sh" -o -iname "README*" \) \
            -not -name "meta-context.sh" \
            -not -path "./.git/*" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
    # Optionally include test files if requested.
    if $INCLUDE_TESTS; then
         test_files=$(find . -type f -iname "*.bats" \
            -not -name "meta-context.sh" \
            -not -path "./.git/*" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
         files="$files $test_files"
    fi
    if [ -d "rust" ]; then
         echo "Including Rust source files from rust in the context."
         rust_files=$(find rust -type f -iname "*.rs")
         files="$files $rust_files"
    fi
else
    # Default: rust only.
    echo "Including only Rust source files from rust in the context."
    files=$(find rust -type f -iname "*.rs")
fi

# Always include Cargo.toml files across the repository.
cargo_files=$(find . -type f -name "Cargo.toml" -not -path "./.git/*")
if [ -n "$cargo_files" ]; then
    echo "Including all Cargo.toml files in the context."
    files="$files $cargo_files"
fi

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
      # For Rust files: if --include-tests is not set, filter out inline tests.
      if [[ "$file" == *.rs ]] && ! $INCLUDE_TESTS; then
          filter_rust_tests "$file"
          echo -e "\n// Note: rust file unit tests not shown here for brevity."
      else
          cat "$file"
      fi
      echo -e "\n"
    } >> "$temp_context"
done

# Append a horizontal dashed line and a new line.
{
  echo "--------------------------------------------------"
  echo ""
} >> "$temp_context"

# Copy the final context to the clipboard using pbcopy.
pbcopy < "$temp_context"

echo "--------------------------------------------------"
echo "Success: Meta context has been copied to the clipboard."
echo "--------------------------------------------------"

# Clean up the temporary file.
rm "$temp_context"
