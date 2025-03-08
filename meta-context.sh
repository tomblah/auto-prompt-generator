#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects Rust source files (and Cargo.toml files)
# in the repository and copies them to the clipboard.
#
# Options:
#   --unit-tests <crate>
#         Extract only the unit tests from the crate’s src/lib.rs and/or src/main.rs.
#
#   --integration-tests <crate>
#         Include all files (integration tests) from the crate’s tests/ directory.
#
# If no option is provided, the default behavior is to include the
# Rust source files in the rust/ directory (excluding integration test files)
# and all Cargo.toml files.
##########################################

# Function to filter out inline Rust test blocks (used in default mode).
filter_rust_tests() {
    awk '
    BEGIN { in_tests=0; brace_count=0 }
    {
        if (in_tests == 0 && $0 ~ /^[[:space:]]*#\[cfg\(test\)\]/) {
            in_tests = 1;
            next;
        }
        if (in_tests == 1 && $0 ~ /^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{/) {
            brace_count = 1;
            next;
        }
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

# Function to extract only the unit test blocks from a Rust source file.
extract_unit_tests() {
    awk '
    BEGIN { capture=0; brace_count=0 }
    {
        # Look for the beginning of a test configuration.
        if (!capture && $0 ~ /^[[:space:]]*#\[cfg\(test\)\]/) {
            capture=1;
            next;
        }
        # When inside the test block, look for the tests module.
        if (capture && $0 ~ /^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{/) {
            brace_count = 1;
            print $0;
            next;
        }
        # If we are inside the tests module, print all lines.
        if (capture && brace_count > 0) {
            print $0;
            n = gsub(/\{/, "{");
            m = gsub(/\}/, "}");
            brace_count += n - m;
            if (brace_count <= 0) {
                capture = 0;
                brace_count = 0;
            }
            next;
        }
    }
    ' "$1"
}

# Determine the mode and optional crate name.
MODE="default"   # default: include rust/ files and Cargo.toml
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
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
fi

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine the repository root (assumes you're in a Git repository).
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

files=""

if [[ "$MODE" == "default" ]]; then
    echo "Including only Rust source files from the rust/ directory (excluding integration tests)."
    # Exclude files in any "tests" directory.
    files=$(find rust -type f -iname "*.rs" -not -path "*/tests/*")
    # Always include Cargo.toml files across the repository.
    cargo_files=$(find . -type f -name "Cargo.toml" -not -path "./.git/*")
    if [ -n "$cargo_files" ]; then
        echo "Including all Cargo.toml files in the context."
        files="$files $cargo_files"
    fi

elif [[ "$MODE" == "unit" ]]; then
    echo "Extracting unit tests for crate: $CRATE"
    if [ ! -d "$CRATE" ]; then
        echo "Error: Crate directory '$CRATE' does not exist." >&2
        exit 1
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
    if [ ! -d "$CRATE/tests" ]; then
        echo "Error: Integration tests directory '$CRATE/tests' does not exist." >&2
        exit 1
    fi
    files=$(find "$CRATE/tests" -type f)
    if [[ -z "$files" ]]; then
        echo "Error: No test files found in '$CRATE/tests'." >&2
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
      if [[ "$MODE" == "unit" ]]; then
          echo "Unit tests extracted from $file:"
      elif [[ "$MODE" == "integration" ]]; then
          echo "Integration test file $file contents:"
      else
          echo "The contents of $file is as follows:"
      fi
      echo "--------------------------------------------------"
      if [[ "$MODE" == "default" && "$file" == *.rs ]]; then
          filter_rust_tests "$file"
          echo -e "\n// Note: rust file unit tests not shown here for brevity."
      elif [[ "$MODE" == "unit" && "$file" == *.rs ]]; then
          extract_unit_tests "$file"
          echo -e "\n// Note: Only unit test blocks have been extracted from this file."
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
