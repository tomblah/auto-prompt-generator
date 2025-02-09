#!/bin/bash
# find-definition-files.sh
#
# This function searches for Swift files that contain definitions for any of the types
# listed in a given types file. It now builds a combined regex for all types to reduce
# the number of find/grep executions.
#
# Usage: find-definition-files <types_file> <root>
#
# Output:
#   On success: prints the path to a temporary file containing a list of Swift files
#   where definitions were found.
find-definition-files() {
    local types_file="$1"
    local root="$2"

    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    # Get the search roots (optimized by the new get-search-roots.sh).
    local search_roots
    search_roots=$("$script_dir/get-search-roots.sh" "$root")

    local temp_found
    temp_found=$(mktemp)

    # Build a combined regex: join all type names with "|"
    # (Assumes that type names are simple and need no extra escaping.)
    local types_regex
    types_regex=$(paste -sd '|' "$types_file")

    # For each search root, perform one find command using the combined regex.
    for sr in $search_roots; do
         find "$sr" -type f -name "*.swift" -not -path "*/.build/*" \
             -exec grep -lE "\\b(class|struct|enum|protocol|typealias)\\s+($types_regex)\\b" {} \; >> "$temp_found" || true
    done

    # Deduplicate the found files.
    local final_found
    final_found=$(mktemp)
    sort -u "$temp_found" > "$final_found"
    rm -f "$temp_found"

    echo "$final_found"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 2 ]; then
        echo "Usage: $0 <types_file> <root>" >&2
        exit 1
    fi
    find-definition-files "$1" "$2"
fi
