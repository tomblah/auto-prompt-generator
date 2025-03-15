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
#   --include-readme
#         Additionally include any README files (e.g. README, README.md, README.txt)
#         from the repository root.
#
# Default (no option): include Rust source files in all crates’ src directories (excluding tests)
# and all Cargo.toml files.
##########################################

# Default settings
MODE="default"
CRATE=""
INCLUDE_README=false

# Process all command-line arguments.
while [[ $# -gt 0 ]]; do
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
        --include-readme)
            INCLUDE_README=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Determine the directory where this script resides and the repository root.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

files=""

if [[ "$MODE" == "default" ]]; then
    echo "Including Rust source files from all crates' src directories (excluding tests) and all Cargo.toml files."
    files=$(find crates -type f -path "*/src/*.rs")
    cargo_files=$(find . -type f -name "Cargo.toml" -not -path "./.git/*")
    if [ -n "$cargo_files" ]; then
        echo "Including all Cargo.toml files in the context."
        files="$files $cargo_files"
    fi

elif [[ "$MODE" == "unit" ]]; then
    echo "Extracting unit tests for crate: $CRATE"
    if [ ! -d "$CRATE" ]; then
        if [ -d "crates/$CRATE" ]; then
            CRATE="crates/$CRATE"
        else
            echo "Error: Crate directory '$CRATE' does not exist." >&2
            exit 1
        fi
    fi
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

if [ "$INCLUDE_README" = true ]; then
    echo "Including README files in the context."
    for readme in README README.md README.txt; do
        if [ -f "$readme" ]; then
            files="$files $readme"
        fi
    done
fi

echo "--------------------------------------------------"
echo "Files to include in the meta-context prompt:"
for file in $files; do
    echo " - $file"
done
echo "--------------------------------------------------"

temp_context=$(mktemp)

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
          awk '
          BEGIN {in_test=0; brace_count=0}
          /^\s*#\[cfg\(test\)\]/ { in_test=1; next }
          in_test && /^\s*mod[[:space:]]+tests[[:space:]]*\{/ { brace_count=1; next }
          in_test {
              n = gsub(/\{/, "{")
              m = gsub(/\}/, "}")
              brace_count += n - m
              if(brace_count <= 0) { in_test=0 }
              next
          }
          { print }
          ' "$file"
          echo -e "\n// Note: Inline unit tests have been removed for brevity."
      elif [[ "$MODE" == "unit" && "$file" == *.rs ]]; then
          awk '
          BEGIN { capture=0; brace_count=0 }
          /^\s*#\[cfg\(test\)\]/ { capture=1; next }
          capture && /^\s*mod[[:space:]]+tests[[:space:]]*\{/ { brace_count=1; print; next }
          capture {
              print;
              n = gsub(/\{/, "{");
              m = gsub(/\}/, "}");
              brace_count += n - m;
              if(brace_count <= 0) { capture=0 }
              next
          }
          ' "$file"
          echo -e "\n// Note: Only unit test blocks have been extracted from this file."
      else
          cat "$file"
      fi
      echo -e "\n"
    } >> "$temp_context"
done

###############################################################################
# NEW SECTION: Append differences from main.
#
# For each file that differs from main, output a single CTA line that combines
# the header and the first diff line. Then, if a block is found (via scanning forward
# until an opening brace is encountered), output that block on its own (with extra newlines).
#
# If the diff line’s final character is alphanumeric, append a colon to the CTA line.
###############################################################################

echo "--------------------------------------------------" >> "$temp_context"

# Global counter for diff files.
DIFF_FILE_COUNT=0

# Function to extract a block of code from a file starting at a given line.
extract_block() {
    local file="$1"
    local start_line="$2"
    local brace_count=0
    local block=""
    local line

    line=$(sed -n "${start_line}p" "$file")
    if [[ ! $line =~ \{ ]]; then
        return
    fi

    for ((i=start_line; ; i++)); do
        line=$(sed -n "${i}p" "$file")
        if [ -z "$line" ]; then
            break
        fi
        block+="$line"$'\n'
        open_count=$(echo "$line" | grep -o "{" | wc -l)
        close_count=$(echo "$line" | grep -o "}" | wc -l)
        brace_count=$((brace_count + open_count - close_count))
        if [ $brace_count -le 0 ]; then
            break
        fi
    done
    echo "$block"
}

# Function to extract the first diff line (with extra block, if any) and output a combined CTA line.
extract_diff_with_block() {
    local file="$1"
    local diff_output
    local current_line=0
    local first_diff_found=false
    local header_diff_line=""
    local extra_block=""

    diff_output=$(git diff --unified=0 main -- "$file")
    if [ -z "$diff_output" ]; then
        return
    fi

    # Determine the CTA header prefix.
    local cta_prefix=""
    if [ "$DIFF_FILE_COUNT" -eq 0 ]; then
        cta_prefix="Can you have a look in $file, in particular, "
    else
        cta_prefix="Also, can you look in $file, in particular, "
    fi

    # Process diff output.
    while IFS= read -r line; do
        if [[ $line =~ ^\+\+ ]]; then
            continue
        fi
        if [[ $line =~ ^@@ ]]; then
            if [[ $line =~ \+([0-9]+) ]]; then
                current_line=${BASH_REMATCH[1]}
                current_line=$((current_line - 1))
            fi
        elif [[ $line =~ ^\+ ]]; then
            current_line=$((current_line + 1))
            # Process the diff line.
            local diff_line="${line:1}"
            diff_line="$(echo "$diff_line" | sed 's#// TODO: -##g')"
            diff_line="$(echo "$diff_line" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
            diff_line="$(echo "$diff_line" | sed 's#^//[[:space:]]*##')"
            if [ -n "$diff_line" ] && [ "$first_diff_found" = false ]; then
                header_diff_line="$diff_line"
                first_diff_found=true
            fi
            # Scan forward for an extra block.
            local block_start_line=$((current_line + 1))
            while true; do
                local next_line
                next_line=$(sed -n "${block_start_line}p" "$file")
                if [ -z "$next_line" ]; then
                    break
                fi
                if [[ $next_line =~ \{ ]]; then
                    extra_block=$(extract_block "$file" "$block_start_line")
                    break
                fi
                block_start_line=$((block_start_line + 1))
            done
            # Process only the first diff line.
            break
        fi
    done <<< "$diff_output"

    # Only proceed if we found a diff line.
    if [ -n "$header_diff_line" ]; then
        local combined_line="${cta_prefix}${header_diff_line}"
        # If the last character of header_diff_line is alphanumeric, append a colon.
        if [[ "$header_diff_line" =~ [[:alnum:]]$ ]]; then
            combined_line="${combined_line}:"
        fi
        echo "$combined_line" >> "$temp_context"
        # If an extra block was found, output it with an extra newline before and after.
        if [ -n "$extra_block" ]; then
            echo "" >> "$temp_context"
            echo "$extra_block" >> "$temp_context"
            echo "" >> "$temp_context"
        fi
    fi
}

for file in $files; do
    if git diff main -- "$file" | grep -q .; then
         extract_diff_with_block "$file"
         DIFF_FILE_COUNT=$((DIFF_FILE_COUNT + 1))
    fi
done

###############################################################################
# End NEW SECTION
###############################################################################

pbcopy < "$temp_context"

echo "--------------------------------------------------"
echo "Success: Meta context (with differences from main) has been copied to the clipboard."
echo "--------------------------------------------------"

rm "$temp_context"
