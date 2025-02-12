#!/bin/bash
# find-referencing-files.sh
#
# This helper defines a function to search for files (Swift, Objective-C header,
# and Objective-C implementation) that reference a given type name. It returns a
# temporary file containing a list of matching files.
#
# Usage (when sourcing):
#   find_referencing_files <type_name> <search_root>
#
# When executed directly, it performs a quick test.

# Source file-types.sh to import allowed file type includes.
source "$(dirname "${BASH_SOURCE[0]}")/file-types.sh"

find_referencing_files() {
    local type_name="$1"
    local search_root="$2"

    local temp_file
    temp_file=$(mktemp)

    # Search for occurrences of the type name as a whole word in files using the allowed
    # file types, excluding common build directories.
    grep -rlE "\\b$type_name\\b" "${ALLOWED_GREP_INCLUDES[@]}" "$search_root" \
         --exclude-dir=Pods --exclude-dir=.build > "$temp_file" 2>/dev/null

    echo "$temp_file"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 2 ]; then
         echo "Usage: $0 <type_name> <search_root>" >&2
         exit 1
    fi
    find_referencing_files "$1" "$2"
fi
