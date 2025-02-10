#!/bin/bash
# filter-files.sh
#
# This function filters a list of Swift file paths when slim mode is enabled.
# It always includes the TODO file and excludes files whose names match
# certain keywords (e.g. ViewController, Manager, Presenter, Router, Interactor,
# Configurator, DataSource, Delegate, or View).
#
# Usage: filter-files_for_slim_mode <todo_file> <found_files_file>
#   <todo_file> is the file containing the TODO.
#   <found_files_file> is a file listing paths to candidate files.
#
# It outputs the path to a temporary file containing the filtered list.
filter-files_for_slim_mode() {
    local todo_file="$1"
    local found_files_file="$2"

    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Starting filtering in filter-files_for_slim_mode" >&2
         echo "[VERBOSE] TODO file: $todo_file" >&2
         echo "[VERBOSE] Candidate file list file: $found_files_file" >&2
    fi

    local filtered_files
    filtered_files=$(mktemp)

    # Always include the file containing the TODO.
    echo "$todo_file" >> "$filtered_files"
    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Added TODO file to filtered list: $todo_file" >&2
    fi

    # Process each file in the found files list.
    while IFS= read -r file; do
        # Skip if it's the TODO file.
        if [ "$file" = "$todo_file" ]; then
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[VERBOSE] Skipping candidate as it matches the TODO file: $file" >&2
            fi
            continue
        fi
        local base
        base=$(basename "$file")
        if [ "${VERBOSE:-false}" = true ]; then
            echo "[VERBOSE] Processing file: $file (basename: $base)" >&2
        fi
        # Exclude files that are likely not models.
        if [[ "$base" =~ (ViewController|Manager|Presenter|Router|Interactor|Configurator|DataSource|Delegate|View) ]]; then
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[VERBOSE] Excluding file based on pattern match: $base" >&2
            fi
            continue
        fi
        echo "$file" >> "$filtered_files"
        if [ "${VERBOSE:-false}" = true ]; then
            echo "[VERBOSE] Including file: $file" >&2
        fi
    done < "$found_files_file"

    if [ "${VERBOSE:-false}" = true ]; then
         local count
         count=$(wc -l < "$filtered_files")
         echo "[VERBOSE] Total files in filtered list: $count" >&2
    fi

    echo "$filtered_files"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 2 ]; then
         echo "Usage: $0 <todo_file> <found_files_file>" >&2
         exit 1
    fi
    filter-files_for_slim_mode "$1" "$2"
fi
