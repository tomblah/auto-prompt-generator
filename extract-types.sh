#!/bin/bash
# extract-types.sh
#
# This function extracts potential type names (classes, structs, enums, etc.)
# from a given Swift file. It processes the file in several stages:
#
#   1. Preprocessing: Replace non-alphanumeric characters with whitespace.
#   2. Stage 0: Trim leading spaces.
#   3. Stage 1: Remove import lines.
#   4. Stage 2: Remove comment lines.
#   5. Stage 3: Extract capitalized words (and types within brackets),
#               then sort and remove duplicates.
#
# Usage: extract-types <swift_file>
#
# Output:
#   On success: prints the path to a temporary file containing a sorted,
#               unique list of potential type names.
#
#   All intermediate temporary files (except the final output) are cleaned up.
extract-types() {
    local swift_file="$1"
    
    # Create a temporary directory for all intermediate files.
    local tempdir
    tempdir=$(mktemp -d)

    # Define paths for intermediate files inside the temporary directory.
    local temp_preprocess="$tempdir/temp_preprocess"
    local temp_stage0="$tempdir/temp_stage0"
    local temp_stage1="$tempdir/temp_stage1"
    local temp_stage2="$tempdir/temp_stage2"
    local types_file="$tempdir/types_file"

    # Set a trap to ensure the temporary directory is removed if the function exits prematurely.
    trap 'rm -rf "$tempdir"' EXIT

    # Preprocessing: Replace all non-alphanumeric characters with whitespace.
    awk '{gsub(/[^a-zA-Z0-9]/, " "); print}' "$swift_file" > "$temp_preprocess"

    # Stage 0: Trim leading spaces from each line.
    awk '{$1=$1; print}' "$temp_preprocess" > "$temp_stage0"

    # Stage 1: Remove lines starting with "import".
    awk '!/^import /' "$temp_stage0" > "$temp_stage1"

    # Stage 2 (modified): Instead of filtering out comment lines, pass them through.
    cp "$temp_stage1" "$temp_stage2"

    # Stage 3: Scan for potential type names:
    #         - Words that start with a capital letter.
    #         - Words within square brackets (e.g., [TypeName]).
    awk '
    {
        for(i = 1; i <= NF; i++) {
            if ($i ~ /^[A-Z][A-Za-z0-9]+$/) {
                print $i
            } else if ($i ~ /\[[A-Z][A-Za-z0-9]+\]/) {
                gsub(/\[|\]/, "", $i)
                print $i
            }
        }
    }' "$temp_stage2" | sort | uniq > "$types_file"

    # Copy the final types file to a new temporary file outside of tempdir.
    local final_types_file
    final_types_file=$(mktemp)
    cp "$types_file" "$final_types_file"

    # Clean up: Remove the temporary directory and its contents.
    rm -rf "$tempdir"
    trap - EXIT

    # Output the path to the final file containing the sorted, unique list of types.
    echo "$final_types_file"
}

# Allow running this file directly for quick manual testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <swift_file>" >&2
        exit 1
    fi
    extract-types "$1"
fi
