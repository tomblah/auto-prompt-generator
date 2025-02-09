#!/bin/bash
# find_definition_files.sh
#
# This function searches for Swift files that contain definitions for each type
# listed in a given types file. It looks for definitions of classes, structs, enums,
# protocols, or typealiases matching the type names.
#
# Usage: find_definition_files <types_file> <root>
#
# Output:
#   On success: prints the path to a temporary file containing a list of Swift files
#   where definitions were found.
find_definition_files() {
    local types_file="$1"
    local root="$2"

    # Determine the directory where this script resides so we can reliably source get_search_roots.sh.
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    # Get all search roots (the given root plus any Swift package directories)
    local search_roots
    search_roots=$("$script_dir/get_search_roots.sh" "$root")

    # Create a temporary directory for intermediate results.
    local tempdir
    tempdir=$(mktemp -d)
    local temp_found="$tempdir/found_files.txt"
    touch "$temp_found"

    # For each type in the types file, search in each of the search roots.
    while IFS= read -r TYPE; do
        for sr in $search_roots; do
            # Use find to locate *.swift files, excluding those in any .build directory
            find "$sr" -type f -name "*.swift" -not -path "*/.build/*" \
                -exec grep -lE "\\b(class|struct|enum|protocol|typealias)\\s+$TYPE\\b" {} \; >> "$temp_found" || true
        done
    done < "$types_file"

    # Copy and deduplicate results to a new temporary file outside the temp directory.
    local final_found
    final_found=$(mktemp)
    sort -u "$temp_found" > "$final_found"

    # Clean up the temporary directory.
    rm -rf "$tempdir"

    # Output the path to the final file.
    echo "$final_found"
}

# Allow direct execution for a quick test.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 2 ]; then
        echo "Usage: $0 <types_file> <root>" >&2
        exit 1
    fi
    find_definition_files "$1" "$2"
fi
