#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects Rust source files (and Cargo.toml files)
# in the repository and copies them to the clipboard.
#
# Supported options:
#
#   --unit-tests <crate>
#         Extract only the unit tests from the crate’s src/lib.rs and/or src/main.rs.
#
#   --integration-tests <crate>
#         Include all files (integration tests) from the crate’s tests/ directory.
#
#   --integration-tests-swift <crate>
#         Include only integration test files whose names contain "swift" (case insensitive)
#         from the crate’s tests/ directory.
#
#   --integration-tests-js <crate>
#         Include only integration test files whose names contain "js" or "javascript" (case insensitive)
#         from the crate’s tests/ directory.
#
# Default (no option): include Rust source files in all crates’ src directories (excluding tests)
# and all Cargo.toml files.
##########################################

MODE="default"
CRATE=""

if [[ $# -gt 0 ]]; then
    case "$1" in
        --unit-tests)
            if [[ $# -lt 2 ]]; then
                echo "Error: --unit-tests requires a crate name." >&2
                exit 1
            fi
            MODE="unit"
            CRATE="$2"
            shift 2
            ;;
        --integration-tests)
            if [[ $# -lt 2 ]]; then
                echo "Error: --integration-tests requires a crate name." >&2
                exit 1
            fi
            MODE="integration"
            CRATE="$2"
            shift 2
            ;;
        --integration-tests-swift)
            if [[ $# -lt 2 ]]; then
                echo "Error: --integration-tests-swift requires a crate name." >&2
                exit 1
            fi
            MODE="integration-swift"
            CRATE="$2"
            shift 2
            ;;
        --integration-tests-js)
            if [[ $# -lt 2 ]]; then
                echo "Error: --integration-tests-js requires a crate name." >&2
                exit 1
            fi
            MODE="integration-js"
            CRATE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
fi

# Determine the directory where this script resides and the repository root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

files=""

if [[ "$MODE" == "default" ]]; then
    echo "Including Rust source files from all crates' src directories (excluding tests) and all Cargo.toml files."
    # Look for .rs files in any crates/*/src folder.
    files=$(find crates -type f -path "*/src/*.rs")
    # Find all Cargo.toml files (both in the root and in each crate)
    cargo_files=$(find . -type f -name "Cargo.toml" -not -path "./.git/*")
    if [ -n "$cargo_files" ]; then
        echo "Including all Cargo.toml files in the context."
        files="$files $cargo_files"
    fi

elif [[ "$MODE" == "unit" ]]; then
    echo "Extracting unit tests for crate: $CRATE"
    # If the provided CRATE does not exist as given, try prepending 'crates/'
    if [ ! -d "$CRATE" ]; then
        if [ -d "crates/$CRATE" ]; then
            CRATE="crates/$CRATE"
        else
            echo "Error: Crate directory '$CRATE' does not exist." >&2
            exit 1
        fi
    fi
    # In unit test mode, include only the test blocks from src/lib.rs and/or src/main.rs.
    if [ -f "$CRATE/src/lib.rs" ]; then
        files="$files $CRATE/src/lib.rs"
    fi
    if [ -f "$CRATE/src/main.rs" ]; then
        files="$files $CRATE/src/main.rs"
    fi
    if [[ -z "$files" ]]; then
        echo "Error: No src/lib.rs or src/main.rs found in crate '$CRATE'." >&2
        exit 1
    fi

elif [[ "$MODE" == "integration" ]]; then
    echo "Including integration tests for crate: $CRATE"
    if [ ! -d "$CRATE" ]; then
        if [ -d "crates/$CRATE" ]; then
            CRATE="crates/$CRATE"
        else
            echo "Error: Crate directory '$CRATE' does not exist." >&2
            exit 1
        fi
    fi
    if [ ! -d "$CRATE/tests" ]; then
        echo "Error: Integration tests directory '$CRATE/tests' does not exist." >&2
        exit 1
    fi
    files=$(find "$CRATE/tests" -type f)
    if [[ -z "$files" ]]; then
        echo "Error: No test files found in '$CRATE/tests'." >&2
        exit 1
    fi

elif [[ "$MODE" == "integration-swift" ]]; then
    echo "Including Swift integration tests for crate: $CRATE"
    if [ ! -d "$CRATE" ]; then
        if [ -d "crates/$CRATE" ]; then
            CRATE="crates/$CRATE"
        else
            echo "Error: Crate directory '$CRATE' does not exist." >&2
            exit 1
        fi
    fi
    if [ ! -d "$CRATE/tests" ]; then
        echo "Error: Integration tests directory '$CRATE/tests' does not exist." >&2
        exit 1
    fi
    files=$(find "$CRATE/tests" -type f -iname '*swift*')
    if [[ -z "$files" ]]; then
        echo "Error: No Swift test files found in '$CRATE/tests'." >&2
        exit 1
    fi

elif [[ "$MODE" == "integration-js" ]]; then
    echo "Including JavaScript integration tests for crate: $CRATE"
    if [ ! -d "$CRATE" ]; then
        if [ -d "crates/$CRATE" ]; then
            CRATE="crates/$CRATE"
        else
            echo "Error: Crate directory '$CRATE' does not exist." >&2
            exit 1
        fi
    fi
    if [ ! -d "$CRATE/tests" ]; then
        echo "Error: Integration tests directory '$CRATE/tests' does not exist." >&2
        exit 1
    fi
    files=$(find "$CRATE/tests" -type f \( -iname '*js*' -o -iname '*javascript*' \))
    if [[ -z "$files" ]]; then
        echo "Error: No JavaScript test files found in '$CRATE/tests'." >&2
        exit 1
    fi
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
      if [[ "$MODE" == "unit" && "$file" == *.rs ]]; then
          echo "Unit tests extracted from $file:"
      elif [[ "$MODE" == "integration" || "$MODE" == "integration-swift" || "$MODE" == "integration-js" ]]; then
          echo "Integration test file $file contents:"
      else
          echo "The contents of $file:"
      fi
      echo "--------------------------------------------------"
      if [[ "$MODE" == "default" && "$file" == *.rs ]]; then
          # Filter out inline unit test blocks from Rust source files.
          awk '
          BEGIN {in_test=0; brace_count=0}
          # When encountering a cfg(test) attribute, begin skipping.
          /^\s*#\[cfg\(test\)\]/ { in_test=1; next }
          # When entering the test module, start counting braces.
          in_test && /^\s*mod tests\s*\{/ { brace_count=1; next }
          # If inside a test block, update brace count and skip lines.
          in_test {
              n = gsub(/{/,"{")
              m = gsub(/}/,"}")
              brace_count += n - m
              if (brace_count <= 0) { in_test=0 }
              next
          }
          { print }
          ' "$file"
          echo -e "\n// Note: Inline unit tests have been removed for brevity."
      elif [[ "$MODE" == "unit" && "$file" == *.rs ]]; then
          # Extract only the unit test blocks.
          awk '
          BEGIN { capture=0; brace_count=0 }
          {
              if (!capture && $0 ~ /^[[:space:]]*#\[cfg\(test\)\]/) {
                  capture=1;
                  next;
              }
              if (capture && $0 ~ /^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{/) {
                  brace_count=1;
                  print;
                  next;
              }
              if (capture && brace_count > 0) {
                  print;
                  n = gsub(/\{/, "{");
                  m = gsub(/\}/, "}");
                  brace_count += n - m;
                  if (brace_count <= 0) {
                      capture=0;
                      brace_count=0;
                  }
                  next;
              }
              if (!capture)
                  print;
          }
          ' "$file"
          echo -e "\n// Note: Only unit test blocks have been extracted from this file."
      else
          cat "$file"
      fi
      echo -e "\n"
    } >> "$temp_context"
done

# Append a horizontal dashed line.
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
