#!/bin/bash
# exclude-files.sh
#
# This function removes file paths from a list (in a temporary file) if their
# basenames match any of the exclusion patterns provided.
#
# Usage: filter_excluded_files <found_files_file> <exclusion1> [<exclusion2> ...]
#
# It outputs the path to a new temporary file containing the filtered list.
filter_excluded_files() {
    local found_files_file="$1"
    shift
    local exclusions=("$@")
    local filtered_file
    filtered_file=$(mktemp)

    # Process each file in the found files list.
    while IFS= read -r file; do
        local base
        base=$(basename "$file" | xargs)
        local exclude=false
        for pattern in "${exclusions[@]}"; do
            if [[ "$base" == "$pattern" ]]; then
                exclude=true
                break
            fi
        done
        if [ "$exclude" = false ]; then
            echo "$file" >> "$filtered_file"
        fi
    done < "$found_files_file"

    echo "$filtered_file"
}
