#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects the contents of various files in the repository and copies them to the clipboard.
#
# Options:
#   --include-tests  : Includes .bats test files along with .sh and README* files.
#                      For Rust files, this option leaves inline test code intact.
#   --tests-only     : Includes only .bats test files.
#   --rust-only      : Includes only Rust source files (under the rust directory).
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

# Optionally, determine the repository root (assumes you're in a Git repository).
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
            -not -path "./.git/*" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
else
    # For non-rust-only modes, start with shell and README files.
    if $INCLUDE_TESTS; then
        echo "Including .bats files along with .sh and README* files in the context."
        files=$(find . -type f \( -iname "*.sh" -o -iname "README*" -o -iname "*.bats" \) \
            -not -name "meta-context.sh" \
            -not -path "./.git/*" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
    else
        files=$(find . -type f \( -iname "*.sh" -o -iname "README*" \) \
            -not -name "meta-context.sh" \
            -not -path "./.git/*" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
    fi

    # Additionally, always include Rust source files if the rust directory exists.
    if [ -d "rust" ]; then
        echo "Including Rust source files from rust in the context."
        rust_files=$(find rust -type f -iname "*.rs")
        files="$files $rust_files"
    fi
fi

# --------------------------------------------------
# Include all Cargo.toml files across the repository.
cargo_files=$(find . -type f -name "Cargo.toml" -not -path "./.git/*")
if [ -n "$cargo_files" ]; then
    echo "Including all Cargo.toml files in the context."
    files="$files $cargo_files"
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

# Append the final custom message based on the option provided.
if $TESTS_ONLY; then
    {
      echo "--------------------------------------------------"
      echo -e "Can you look through these tests and add unit tests to cover the functionality we've added.\n\nLet's lean towards appending to existing files where it makes sense to do so. However, if we've created a new script, it might make sense to create a new test file for it. And be sure to echo out the entire test file with the added test cases."
    } >> "$temp_context"
elif ! $RUST_ONLY; then
    {
      echo "--------------------------------------------------"
      echo -e "\n"
    } >> "$temp_context"
fi

# Copy the final context to the clipboard using pbcopy.
pbcopy < "$temp_context"

echo "--------------------------------------------------"
echo "Success: Meta context has been copied to the clipboard."
echo "--------------------------------------------------"

# Clean up the temporary file.
rm "$temp_context"
