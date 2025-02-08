#!/bin/bash
# find_definition_files.sh
#
# This function searches for Swift files that contain definitions for each type
# listed in a given types file. It looks for definitions of classes, structs, enums,
# protocols, or typealiases matching the type names.
#
# Usage: find_definition_files <types_file> <git_root>
#
# Output:
#   On success: prints the path to a temporary file containing a list of Swift files
#   where definitions were found.
find_definition_files() {
    local types_file="$1"
    local git_root="$2"

    # Create a temporary directory to store the intermediate file.
    local tempdir
    tempdir=$(mktemp -d)

    local temp_found="$tempdir/found_files.txt"

    # For each type listed in the types file, search for matching definitions.
    while IFS= read -r TYPE; do
        grep -rwlE --include="*.swift" "\\b(class|struct|enum|protocol|typealias)\\s+$TYPE\\b" "$git_root" >> "$temp_found" || true
    done < "$types_file"

    # Copy the final results to a new temporary file outside of the temporary directory.
    local final_found
    final_found=$(mktemp)
    cp "$temp_found" "$final_found"

    # Clean up the temporary directory and its contents.
    rm -rf "$tempdir"

    # Output the path to the final file.
    echo "$final_found"
}

# Allow running this file directly for a quick manual test.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 2 ]; then
        echo "Usage: $0 <types_file> <git_root>" >&2
        exit 1
    fi
    find_definition_files "$1" "$2"
fi
