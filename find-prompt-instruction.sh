#!/bin/bash
# find-prompt-instruction.sh
#
# This function looks for a file that contains a TODO instruction marked by:
#   - "// TODO: - "
#
# It supports multiple file types (Swift, Objective-C header, and Objective-C implementation)
# by using the allowed file type filters defined in file-types.sh.
#
# If no such file exists, it outputs an error message.
# If more than one file contains the instruction, it chooses the file which is most
# recently edited and logs a message listing the ignored TODO files.
#
# Usage: find-prompt-instruction <search_directory>
#
# Outputs:
#   On success: prints the file path (for further processing) of the chosen instruction.
#   On failure: prints an error message to stderr and returns a non-zero exit code.
#
# Note: If the global variable VERBOSE is set to "true" (for example via --verbose in generate-prompt.sh),
# this function will output additional debug logging to stderr.

# Source file-types.sh to include allowed file extensions.
source "$(dirname "${BASH_SOURCE[0]}")/file-types.sh"

find-prompt-instruction() {
    local search_dir="$1"
    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Starting search in directory: $search_dir" >&2
    fi

    # Pattern matching only "// TODO: - " (with trailing space)
    local grep_pattern='// TODO: - '

    # Read all matching file paths into an array.
    local files_array=()
    while IFS= read -r line; do
        files_array+=("$line")
    done < <(grep -rlE "$grep_pattern" --exclude-dir=Pods "${ALLOWED_GREP_INCLUDES[@]}" "$search_dir" 2>/dev/null)

    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Found ${#files_array[@]} file(s) matching TODO pattern." >&2
       for file in "${files_array[@]}"; do
            echo "[VERBOSE] Matched file: $file" >&2
       done
    fi

    local file_count="${#files_array[@]}"

    if [ "$file_count" -eq 0 ]; then
        echo "Error: No files found containing '// TODO: - '" >&2
        return 1
    fi

    if [ "$file_count" -eq 1 ]; then
        if [ "${VERBOSE:-false}" = true ]; then
           echo "[VERBOSE] Only one matching file found: ${files_array[0]}" >&2
        fi
        echo "${files_array[0]}"
        return 0
    fi

    # More than one file: determine the one with the most recent modification time.
    local chosen_file="${files_array[0]}"
    local chosen_mod_time
    chosen_mod_time=$(stat -f "%m" "${chosen_file}")
    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Initial chosen file: $chosen_file with modification time $chosen_mod_time" >&2
    fi

    for file in "${files_array[@]}"; do
        local mod_time
        mod_time=$(stat -f "%m" "$file")
        if [ "${VERBOSE:-false}" = true ]; then
           echo "[VERBOSE] Evaluating file: $file with modification time $mod_time" >&2
        fi
        if [ "$mod_time" -gt "$chosen_mod_time" ]; then
            chosen_file="$file"
            chosen_mod_time="$mod_time"
            if [ "${VERBOSE:-false}" = true ]; then
               echo "[VERBOSE] New chosen file: $chosen_file with modification time $chosen_mod_time" >&2
            fi
        fi
    done

    # Build a list of files that were not chosen.
    local ignored_files=()
    for file in "${files_array[@]}"; do
        if [ "$file" != "$chosen_file" ]; then
            ignored_files+=("$file")
        fi
    done

    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Ignoring the following files:" >&2
       for file in "${ignored_files[@]}"; do
           local base
           base=$(basename "$file")
           echo "[VERBOSE] Ignored file: $base" >&2
       done
    fi

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
