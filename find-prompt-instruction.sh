#!/bin/bash
# find-prompt-instruction.sh
#
# This function looks for a Swift file that contains a TODO instruction marked by either:
#   - "// TODO: - "  OR
#   - "// TODO: ChatGPT: "
#
# If no such file exists, it outputs an error message.
# If more than one file contains the instruction, it chooses the file which is most recently edited
# and logs a message that lists (with a separator and line breaks) the ignored TODO files by their base name
# and their actual TODO text.
#
# Usage: find-prompt-instruction <search_directory>
#
# Outputs:
#   On success: prints the file path (for further processing) of the chosen instruction.
#   On failure: prints an error message to stderr and returns a non-zero exit code.
#
# Coverage Warning:
#   - The current test suite does not verify the stderr logging output (e.g. the separator lines,
#     the "Multiple TODO instructions found" message, and the list of ignored files) when multiple
#     matching files are present.
#   - There is no explicit test for handling files that contain multiple matching TODO lines.
#   - The implementation uses macOS’s 'stat -f "%m"' for file modification times; behavior on Linux
#     (which would require 'stat -c "%Y"') is not covered by tests.
#
find-prompt-instruction() {
    local search_dir="$1"
    # Pattern matching either "// TODO: ChatGPT: " or "// TODO: - " (with trailing space)
    local grep_pattern='// TODO: (ChatGPT: |- )'
    
    # Read all matching file paths into an array.
    local files_array=()
    while IFS= read -r line; do
        files_array+=("$line")
    done < <(grep -rlE "$grep_pattern" --exclude-dir=Pods --include "*.swift" "$search_dir" 2>/dev/null)
    
    local file_count="${#files_array[@]}"
    
    if [ "$file_count" -eq 0 ]; then
        echo "Error: No Swift files found containing either '// TODO: - ' or '// TODO: ChatGPT: '" >&2
        return 1
    fi
    
    if [ "$file_count" -eq 1 ]; then
        echo "${files_array[0]}"
        return 0
    fi
    
    # More than one file: determine the one with the most recent modification time.
    local chosen_file="${files_array[0]}"
    # Use macOS's stat syntax; for Linux, replace with: stat -c "%Y" "$file"
    local chosen_mod_time
    chosen_mod_time=$(stat -f "%m" "${chosen_file}")
    
    for file in "${files_array[@]}"; do
        local mod_time
        mod_time=$(stat -f "%m" "$file")
        if [ "$mod_time" -gt "$chosen_mod_time" ]; then
            chosen_file="$file"
            chosen_mod_time="$mod_time"
        fi
    done
    
    # Build a list of files that were not chosen.
    local ignored_files=()
    for file in "${files_array[@]}"; do
        if [ "$file" != "$chosen_file" ]; then
            ignored_files+=("$file")
        fi
    done
    
    # Log the multiple-match message with a single separator and line breaks.
    echo "--------------------------------------------------" >&2
    echo "Multiple TODO instructions found (${file_count} files), the following TODO files were IGNORED:" >&2
    for file in "${ignored_files[@]}"; do
        local base
        base=$(basename "$file")
        # Extract the first matching TODO line from the file.
        local todo_text
        todo_text=$(grep -m 1 -E "$grep_pattern" "$file" | sed 's/^[[:space:]]*//')
        echo "  - ${base}: ${todo_text}" >&2
        echo "--------------------------------------------------" >&2
    done
    
    echo "$chosen_file"
}

# Allow running this file directly for a quick manual test.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <search_directory>" >&2
        exit 1
    fi
    find-prompt-instruction "$1"
fi
