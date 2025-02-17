#!/bin/bash
# find-definition-files.sh
#
# This function searches for files that contain definitions for any of the types
# listed in a given types file by delegating the work to the new Rust binary.
#
# Usage: find-definition-files <types_file> <root>
#
# Output:
#   On success: prints the path to a temporary file containing a list of files
#   where definitions were found.

# (Note: The file-types.sh source is no longer needed because our Rust binary handles allowed extensions.)
# source "$(dirname "${BASH_SOURCE[0]}")/file-types.sh"

find-definition-files() {
    local types_file="$1"
    local root="$2"

    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Running new Rust binary to find definition files" >&2
    fi

    # Call the new Rust binary (adjust the binary name if needed).
    # It expects two arguments: the types file and the root directory.
    local output
    output=$("$script_dir/rust/target/release/find_definition_files" "$types_file" "$root")

    # Write the output (the list of file paths) to a temporary file.
    local temp_found
    temp_found=$(mktemp)
    echo "$output" > "$temp_found"

    if [ "${VERBOSE:-false}" = true ]; then
         local found_count
         found_count=$(wc -l < "$temp_found")
         echo "[VERBOSE] Total unique files found: $found_count" >&2
    fi

    echo "$temp_found"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 2 ]; then
        echo "Usage: $0 <types_file> <root>" >&2
        exit 1
    fi
    find-definition-files "$1" "$2"
fi
