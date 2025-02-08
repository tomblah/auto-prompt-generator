#!/bin/bash
# extract_types.sh
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
# Usage: extract_types <swift_file>
#
# Output:
#   On success: prints the path to a temporary file containing a sorted,
#               unique list of potential type names.
#
#   All intermediate temporary files (except the final output) are cleaned up.
extract_types() {
    local swift_file="$1"

    # Create temporary files for each processing stage.
    local temp_preprocess temp_stage0 temp_stage1 temp_stage2 types_file
    temp_preprocess=$(mktemp)
    temp_stage0=$(mktemp)
    temp_stage1=$(mktemp)
    temp_stage2=$(mktemp)
    types_file=$(mktemp)

    # Preprocessing: Replace all non-alphanumeric characters with whitespace.
    awk '{gsub(/[^a-zA-Z0-9]/, " "); print}' "$swift_file" > "$temp_preprocess"

    # Stage 0: Trim leading spaces from each line.
    awk '{$1=$1; print}' "$temp_preprocess" > "$temp_stage0"

    # Stage 1: Remove lines starting with "import".
    awk '!/^import /' "$temp_stage0" > "$temp_stage1"

    # Stage 2: Remove lines starting with comment markers.
    awk '!/^\/\//' "$temp_stage1" > "$temp_stage2"

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

    # Clean up the intermediate temporary files.
    rm "$temp_preprocess" "$temp_stage0" "$temp_stage1" "$temp_stage2"

    # Output the file containing the sorted, unique list of types.
    echo "$types_file"
}

# Allow running this file directly for quick manual testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <swift_file>" >&2
        exit 1
    fi
    extract_types "$1"
fi
