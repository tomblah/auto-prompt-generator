#!/bin/bash
# filter-files-singular.sh
#
# This function returns a temporary file containing only the Swift file
# that holds the TODO instruction.
#
# Usage: filter_files_singular <todo_file>
filter_files_singular() {
    local todo_file="$1"
    local filtered_file
    filtered_file=$(mktemp)
    echo "$todo_file" > "$filtered_file"
    echo "$filtered_file"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
         echo "Usage: $0 <todo_file>" >&2
         exit 1
    fi
    filter_files_singular "$1"
fi
