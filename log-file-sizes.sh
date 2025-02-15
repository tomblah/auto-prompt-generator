#!/bin/bash
# log-file-sizes.sh
#
# This function logs the sizes of the files (in characters) after applying
# any substring marker filtering. It then prints the list sorted in descending
# order by content size.
#
# Usage:
#   log_file_sizes <file_list>
# where <file_list> is a file containing one file path per line.

# Source the helper that performs substring marker filtering.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/filter-substring-markers.sh"

log_file_sizes() {
    local file_list="$1"
    local temp_output
    temp_output=$(mktemp)

    while IFS= read -r file_path; do
        local file_basename size content
        file_basename=$(basename "$file_path")
        # If the file uses substring markers, filter its content.
        if grep -qE '^[[:space:]]*//[[:space:]]*v[[:space:]]*$' "$file_path"; then
            content=$(filter_substring_markers "$file_path")
        else
            content=$(cat "$file_path")
        fi
        # Calculate size in characters.
        size=$(printf "%s" "$content" | wc -m | tr -d ' ')
        echo "$file_basename ($size)" >> "$temp_output"
    done < "$file_list"

    # Sort the output descending by the number within parentheses.
    sort -t'(' -k2 -nr "$temp_output"
    rm -f "$temp_output"
}

# Allow direct testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
         echo "Usage: $0 <file_list>" >&2
         exit 1
    fi
    log_file_sizes "$1"
fi
