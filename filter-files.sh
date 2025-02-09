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

    local filtered_files
    filtered_files=$(mktemp)

    # Always include the file containing the TODO.
    echo "$todo_file" >> "$filtered_files"

    # Process each file found.
    while IFS= read -r file; do
        # Skip if it's the TODO file.
        if [ "$file" = "$todo_file" ]; then
            continue
        fi
        local base
        base=$(basename "$file")
        # Exclude files that are likely not models.
        if [[ "$base" =~ (ViewController|Manager|Presenter|Router|Interactor|Configurator|DataSource|Delegate|View) ]]; then
            continue
        fi
        echo "$file" >> "$filtered_files"
    done < "$found_files_file"

    echo "$filtered_files"
}
