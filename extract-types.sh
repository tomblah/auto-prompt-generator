#!/bin/bash
# extract-types.sh
#
# This file defines the function extract_types which extracts type names
# (classes, structs, enums, protocols) from a given Swift file by invoking
# SourceKitten and processing its JSON output with jq.
#
# Requirements:
#   - sourcekitten (install via: brew install sourcekitten)
#   - jq (install via: brew install jq)
#
# When sourced, this file simply defines the function.
# When executed directly, it will run extract_types with the provided argument.
#
# Usage (as a command-line tool):
#   ./extract-types.sh <swift_file>
#
# Usage (when sourced):
#   TYPES_FILE=$(extract_types "<swift_file>")
#

set -euo pipefail

extract_types() {
    if [ "$#" -ne 1 ]; then
         echo "Usage: ${FUNCNAME[0]} <swift_file>" >&2
         return 1
    fi

    local swift_file="$1"

    if [ "${VERBOSE:-false}" = true ]; then
         echo "[DEBUG] extract_types called with file: $swift_file" >&2
    fi

    # Check that SourceKitten is installed.
    if ! command -v sourcekitten >/dev/null 2>&1; then
         echo "Error: sourcekitten is not installed. You can install it with 'brew install sourcekitten'." >&2
         return 1
    fi

    # Check that jq is installed.
    if ! command -v jq >/dev/null 2>&1; then
         echo "Error: jq is not installed. You can install it with 'brew install jq'." >&2
         return 1
    fi

    # Ensure the Swift file exists.
    if [ ! -f "$swift_file" ]; then
         echo "Error: File not found: $swift_file" >&2
         return 1
    fi

    # Create a temporary file to hold the type names.
    local output_file
    output_file=$(mktemp)

    if [ "${VERBOSE:-false}" = true ]; then
         echo "[DEBUG] Running SourceKitten on $swift_file" >&2
    fi

    # Use SourceKitten to get the structure, then use jq to:
    #   - Recurse through the JSON hierarchy
    #   - Select items where key.kind indicates a class, struct, enum, or protocol
    #   - Extract the key.name field, then sort and remove duplicates.
    sourcekitten structure --file "$swift_file" | \
      jq -r '
        recurse(.substructure[]?) |
        select(.key.kind? | test("^source.lang.swift.decl\\.(class|struct|enum|protocol)$")) |
        .key.name
      ' | sort -u > "$output_file"

    if [ "${VERBOSE:-false}" = true ]; then
         echo "[DEBUG] Types extracted and saved to: $output_file" >&2
    fi

    echo "$output_file"
}

# If this file is executed directly (not sourced), run extract_types with the provided arguments.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    extract_types "$@"
fi
